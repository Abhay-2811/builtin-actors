use cid::Cid;
use fvm_ipld_encoding::ipld_block::IpldBlock;
use fvm_shared::clock::ChainEpoch;
use fvm_shared::deal::DealID;
use fvm_shared::econ::TokenAmount;
use fvm_shared::error::ExitCode;
use fvm_shared::piece::PaddedPieceSize;
use multihash::Code::Sha2_256;
use multihash::MultihashDigest;
use num_traits::Zero;

use fil_actor_market::ext::miner::{
    PieceInfo, PieceReturn, SectorChanges, SectorContentChangedParams,
};
use fil_actor_market::{DealProposal, Method, NO_ALLOCATION_ID};
use fil_actors_runtime::cbor::serialize;
use fil_actors_runtime::runtime::builtins::Type;
use fil_actors_runtime::test_utils::{expect_abort, MockRuntime, ACCOUNT_ACTOR_CODE_ID};
use fil_actors_runtime::EPOCHS_IN_DAY;
use harness::*;

mod harness;

const START_EPOCH: ChainEpoch = 10;
const END_EPOCH: ChainEpoch = 200 * EPOCHS_IN_DAY;
const MINER_ADDRESSES: MinerAddresses = MinerAddresses {
    owner: OWNER_ADDR,
    worker: WORKER_ADDR,
    provider: PROVIDER_ADDR,
    control: vec![],
};

// These tests share a lot in common with those for BatchActivateDeals,
// as they perform similar functions.

#[test]
fn empty_params() {
    let rt = setup();

    // Empty params
    let changes = vec![];
    let ret = sector_content_changed(&rt, PROVIDER_ADDR, changes).unwrap();
    assert_eq!(0, ret.sectors.len());

    // Sector with no pieces
    let changes =
        vec![SectorChanges { sector: 1, minimum_commitment_epoch: END_EPOCH, added: vec![] }];
    let ret = sector_content_changed(&rt, PROVIDER_ADDR, changes).unwrap();
    assert_eq!(1, ret.sectors.len());
    assert_eq!(0, ret.sectors[0].added.len());
    check_state(&rt);
}

#[test]
fn simple_one_sector() {
    let rt = setup();
    let epoch = rt.set_epoch(START_EPOCH);
    let mut deals = create_deals(&rt, 3);
    deals[2].verified_deal = true;

    let next_allocation_id = 1;
    let datacap_required = TokenAmount::from_whole(deals[2].piece_size.0);
    rt.set_caller(*ACCOUNT_ACTOR_CODE_ID, WORKER_ADDR);
    let deal_ids =
        publish_deals(&rt, &MINER_ADDRESSES, &deals, datacap_required, next_allocation_id);

    let mut pieces = pieces_from_deals(&deal_ids, &deals);
    pieces.reverse();

    let sno = 7;
    let changes = vec![SectorChanges {
        sector: sno,
        minimum_commitment_epoch: END_EPOCH + 10,
        added: pieces,
    }];
    let ret = sector_content_changed(&rt, PROVIDER_ADDR, changes).unwrap();
    assert_eq!(1, ret.sectors.len());
    assert_eq!(3, ret.sectors[0].added.len());
    assert!(ret.sectors[0].added.iter().all(|r| r.code == ExitCode::OK));

    // Deal IDs are stored under the sector, in correct order.
    assert_eq!(deal_ids, get_sector_deal_ids(&rt, &PROVIDER_ADDR, sno));

    // Deal states include allocation IDs from when they were published.
    for id in deal_ids.iter() {
        let state = get_deal_state(&rt, *id);
        assert_eq!(sno, state.sector_number);
        assert_eq!(epoch, state.sector_start_epoch);
        if *id == deal_ids[2] {
            assert_eq!(state.verified_claim, next_allocation_id);
        } else {
            assert_eq!(state.verified_claim, NO_ALLOCATION_ID);
        }
    }
    check_state(&rt);
}

#[test]
fn simple_multiple_sectors() {
    let rt = setup();
    let deals = create_deals(&rt, 3);
    rt.set_caller(*ACCOUNT_ACTOR_CODE_ID, WORKER_ADDR);
    let deal_ids =
        publish_deals(&rt, &MINER_ADDRESSES, &deals, TokenAmount::zero(), NO_ALLOCATION_ID);
    let pieces = pieces_from_deals(&deal_ids, &deals);

    let changes = vec![
        SectorChanges {
            sector: 1,
            minimum_commitment_epoch: END_EPOCH + 10,
            added: pieces[0..1].to_vec(),
        },
        // Same sector referenced twice, it's ok.
        SectorChanges {
            sector: 1,
            minimum_commitment_epoch: END_EPOCH + 10,
            added: pieces[1..2].to_vec(),
        },
        SectorChanges {
            sector: 2,
            minimum_commitment_epoch: END_EPOCH + 10,
            added: pieces[2..3].to_vec(),
        },
    ];
    let ret = sector_content_changed(&rt, PROVIDER_ADDR, changes).unwrap();
    assert_eq!(3, ret.sectors.len());
    assert_eq!(vec![PieceReturn { code: ExitCode::OK, data: vec![] }], ret.sectors[0].added);
    assert_eq!(vec![PieceReturn { code: ExitCode::OK, data: vec![] }], ret.sectors[1].added);
    assert_eq!(vec![PieceReturn { code: ExitCode::OK, data: vec![] }], ret.sectors[2].added);

    // Deal IDs are stored under the right sector, in correct order.
    assert_eq!(deal_ids[0..2], get_sector_deal_ids(&rt, &PROVIDER_ADDR, 1));
    assert_eq!(deal_ids[2..3], get_sector_deal_ids(&rt, &PROVIDER_ADDR, 2));
}

#[test]
fn new_deal_existing_sector() {
    let rt = setup();
    let deals = create_deals(&rt, 3);
    rt.set_caller(*ACCOUNT_ACTOR_CODE_ID, WORKER_ADDR);
    let deal_ids =
        publish_deals(&rt, &MINER_ADDRESSES, &deals, TokenAmount::zero(), NO_ALLOCATION_ID);
    let pieces = pieces_from_deals(&deal_ids, &deals);

    let changes = vec![SectorChanges {
        sector: 1,
        minimum_commitment_epoch: END_EPOCH + 10,
        added: pieces[1..3].to_vec(),
    }];
    sector_content_changed(&rt, PROVIDER_ADDR, changes).unwrap();

    let changes = vec![SectorChanges {
        sector: 1,
        minimum_commitment_epoch: END_EPOCH + 10,
        added: pieces[0..1].to_vec(),
    }];
    sector_content_changed(&rt, PROVIDER_ADDR, changes).unwrap();

    // All deal IDs are stored under the right sector, in correct order.
    assert_eq!(deal_ids[0..3], get_sector_deal_ids(&rt, &PROVIDER_ADDR, 1));
}

#[test]
fn piece_must_match_deal() {
    let rt = setup();
    let deals = create_deals(&rt, 2);

    rt.set_caller(*ACCOUNT_ACTOR_CODE_ID, WORKER_ADDR);
    let deal_ids =
        publish_deals(&rt, &MINER_ADDRESSES, &deals, TokenAmount::zero(), NO_ALLOCATION_ID);
    let mut pieces = pieces_from_deals(&deal_ids, &deals);
    // Wrong CID
    pieces[0].data = Cid::new_v1(0, Sha2_256.digest(&[1, 2, 3, 4]));
    // Wrong size
    pieces[1].size = PaddedPieceSize(1234);
    // Deal doesn't exist
    pieces.push(PieceInfo {
        data: Cid::new_v1(0, Sha2_256.digest(&[1, 2, 3, 4])),
        size: PaddedPieceSize(1234),
        payload: serialize(&1234, "deal id").unwrap().to_vec(),
    });

    let changes =
        vec![SectorChanges { sector: 1, minimum_commitment_epoch: END_EPOCH + 10, added: pieces }];
    let ret = sector_content_changed(&rt, PROVIDER_ADDR, changes).unwrap();
    assert_eq!(1, ret.sectors.len());
    assert_eq!(
        vec![
            PieceReturn { code: ExitCode::USR_ILLEGAL_ARGUMENT, data: vec![] },
            PieceReturn { code: ExitCode::USR_ILLEGAL_ARGUMENT, data: vec![] },
            PieceReturn { code: ExitCode::USR_NOT_FOUND, data: vec![] },
        ],
        ret.sectors[0].added
    );

    check_state(&rt);
}

#[test]
fn failures_isolated() {
    let rt = setup();
    let deals = create_deals(&rt, 4);
    rt.set_caller(*ACCOUNT_ACTOR_CODE_ID, WORKER_ADDR);
    let deal_ids =
        publish_deals(&rt, &MINER_ADDRESSES, &deals, TokenAmount::zero(), NO_ALLOCATION_ID);
    let mut pieces = pieces_from_deals(&deal_ids, &deals);

    // Break second and third pieces.
    pieces[1].size = PaddedPieceSize(1234);
    pieces[2].size = PaddedPieceSize(1234);
    let changes = vec![
        SectorChanges {
            sector: 1,
            minimum_commitment_epoch: END_EPOCH + 10,
            added: pieces[0..2].to_vec(),
        },
        SectorChanges {
            sector: 2,
            minimum_commitment_epoch: END_EPOCH + 10,
            added: pieces[2..3].to_vec(),
        },
        SectorChanges {
            sector: 3,
            minimum_commitment_epoch: END_EPOCH + 10,
            added: pieces[3..4].to_vec(),
        },
    ];

    let ret = sector_content_changed(&rt, PROVIDER_ADDR, changes).unwrap();
    assert_eq!(3, ret.sectors.len());
    // Broken second piece still allows first piece in same sector to activate.
    assert_eq!(
        vec![
            PieceReturn { code: ExitCode::OK, data: vec![] },
            PieceReturn { code: ExitCode::USR_ILLEGAL_ARGUMENT, data: vec![] }
        ],
        ret.sectors[0].added
    );
    // Broken third piece
    assert_eq!(
        vec![PieceReturn { code: ExitCode::USR_ILLEGAL_ARGUMENT, data: vec![] }],
        ret.sectors[1].added
    );
    // Ok fourth piece.
    assert_eq!(vec![PieceReturn { code: ExitCode::OK, data: vec![] }], ret.sectors[2].added);

    // Successful deal IDs are stored under the right sector, in correct order.
    assert_eq!(deal_ids[0..1], get_sector_deal_ids(&rt, &PROVIDER_ADDR, 1));
    assert_eq!(deal_ids[3..4], get_sector_deal_ids(&rt, &PROVIDER_ADDR, 3));
}

#[test]
fn duplicates_rejected() {
    let rt = setup();
    let deals = create_deals(&rt, 1);
    rt.set_caller(*ACCOUNT_ACTOR_CODE_ID, WORKER_ADDR);
    let deal_ids =
        publish_deals(&rt, &MINER_ADDRESSES, &deals, TokenAmount::zero(), NO_ALLOCATION_ID);
    let pieces = pieces_from_deals(&deal_ids, &deals);

    let changes = vec![
        // Same deal twice in one sector change.
        SectorChanges {
            sector: 1,
            minimum_commitment_epoch: END_EPOCH + 10,
            added: vec![pieces[0].clone(), pieces[0].clone()],
        },
        // Same deal again, referencing same sector.
        SectorChanges {
            sector: 1,
            minimum_commitment_epoch: END_EPOCH + 10,
            added: vec![pieces[0].clone()],
        },
        // Same deal again in a different sector.
        SectorChanges {
            sector: 2,
            minimum_commitment_epoch: END_EPOCH + 10,
            added: vec![pieces[0].clone()],
        },
    ];
    let ret = sector_content_changed(&rt, PROVIDER_ADDR, changes).unwrap();
    assert_eq!(3, ret.sectors.len());
    // Succeeds just once.
    assert_eq!(
        vec![
            PieceReturn { code: ExitCode::OK, data: vec![] },
            PieceReturn { code: ExitCode::USR_ILLEGAL_ARGUMENT, data: vec![] },
        ],
        ret.sectors[0].added
    );
    assert_eq!(
        vec![PieceReturn { code: ExitCode::USR_ILLEGAL_ARGUMENT, data: vec![] }],
        ret.sectors[1].added
    );
    assert_eq!(
        vec![PieceReturn { code: ExitCode::USR_ILLEGAL_ARGUMENT, data: vec![] }],
        ret.sectors[2].added
    );

    // Deal IDs are stored under the right sector, in correct order.
    assert_eq!(deal_ids[0..1], get_sector_deal_ids(&rt, &PROVIDER_ADDR, 1));
    assert_eq!(Vec::<DealID>::new(), get_sector_deal_ids(&rt, &PROVIDER_ADDR, 2));
}

#[test]
fn require_miner_caller() {
    let rt = setup();
    let changes = vec![];
    rt.set_caller(*ACCOUNT_ACTOR_CODE_ID, PROVIDER_ADDR); // Not a miner
    rt.expect_validate_caller_type(vec![Type::Miner]);
    let params = SectorContentChangedParams { sectors: changes };

    expect_abort(
        ExitCode::USR_FORBIDDEN,
        rt.call::<fil_actor_market::Actor>(
            Method::SectorContentChangedExported as u64,
            IpldBlock::serialize_cbor(&params).unwrap(),
        ),
    );
}

fn create_deals(rt: &MockRuntime, count: i64) -> Vec<DealProposal> {
    (0..count)
        .map(|i| create_deal(rt, CLIENT_ADDR, &MINER_ADDRESSES, START_EPOCH, END_EPOCH + i, false))
        .collect()
}

fn pieces_from_deals(deal_ids: &[DealID], deals: &[DealProposal]) -> Vec<PieceInfo> {
    deal_ids.iter().zip(deals).map(|(id, deal)| piece_info_from_deal(*id, deal)).collect()
}

// TODO

// - See activate_deals_failures
// - test bad deal ID, serialise, notfound
// - test not pending
// - test bad epoch, provider
// - test already activated
