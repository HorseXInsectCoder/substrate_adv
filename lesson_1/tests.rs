use super::*;
use crate::{mock::*, Error, Proofs};
use frame_support::{assert_noop, assert_ok, BoundedVec};


// 测试创建存证
#[test]
fn create_claim_works() {
	new_test_ext().execute_with(|| {
		// 构造输入信息
		let claim = vec![0, 1];
		assert_ok!(PoeModule::create_claim(Origin::signed(1), claim.clone()));

		let bounded_claim = BoundedVec::<u8, <Test as Config>::MaxClaimLength>::try_from(claim.clone()).unwrap();
		assert_eq!(
			Proofs::<Test>::get(&bounded_claim),
			Some((1, frame_system::Pallet::<Test>::block_number()))
		);
	})
}

// claim失败的场景
#[test]
fn create_claim_failed_when_claim_already() {
	new_test_ext().execute_with(|| {
		// 构造输入信息
		let claim = vec![0, 1];
		let _ = PoeModule::create_claim(Origin::signed(1), claim.clone());

		assert_noop!(
			PoeModule::create_claim(Origin::signed(1), claim.clone()),
			Error::<Test>::ProofAlreadyExist
		);
	})
}

// 撤销存证
#[test]
fn revoke_claim_works() {
	new_test_ext().execute_with(|| {
		let claim = vec![0, 1];
		let _ = PoeModule::create_claim(Origin::signed(1), claim.clone());

		assert_ok!(PoeModule::revoke_claim(Origin::signed(1), claim.clone()));
	})
}

// 撤销不存在的存证
#[test]
fn revoke_claim_failed_when_claim_not_exist() {
	new_test_ext().execute_with(|| {
		let claim = vec![0, 1];

		assert_noop!(
            PoeModule::revoke_claim(Origin::signed(1), claim.clone()),
            Error::<Test>::ClaimNotExist
        );
	})
}


// 撤销非交易发送方的存证
#[test]
fn revoke_claim_failed_when_is_not_owner() {
	new_test_ext().execute_with(|| {
		let claim = vec![0, 1];

		let _ = PoeModule::create_claim(Origin::signed(1), claim.clone());

		assert_noop!(
            PoeModule::revoke_claim(Origin::signed(2), claim.clone()),
            Error::<Test>::NotClaimOwner
        );
	})
}


// 测试转移存证成功
#[test]
fn transfer_claim_works() {
	new_test_ext().execute_with(|| {
		let claim = vec![0, 1];
		let _  = PoeModule::create_claim(Origin::signed(1), claim.clone());

		let bounded_claim = BoundedVec::<u8, <Test as Config>::MaxClaimLength>::try_from(claim.clone()).unwrap();

		assert_ok!(PoeModule::transfer_claim(Origin::signed(1), claim.clone(), 2));

		assert_eq!(Proofs::<Test>::get(&bounded_claim), Some((2, frame_system::Pallet::<Test>::block_number())));

		assert_noop!(
            PoeModule::revoke_claim(Origin::signed(1), claim.clone()),
            Error::<Test>::NotClaimOwner
        );
	})
}

// 测试转移的存证数据不存在
#[test]
fn transfer_claim_failed_when_claim_no_exist() {
	new_test_ext().execute_with(|| {
		let claim = vec![0, 1];
		let _ = PoeModule::create_claim(Origin::signed(1), claim.clone());

		let claim_temp = vec![2, 3];
		assert_noop!(
            PoeModule::transfer_claim(Origin::signed(1), claim_temp.clone(), 2),
            Error::<Test>::ClaimNotExist
        );
	})
}


// 测试转移存证，但转移的发起者非交易发送方
#[test]
fn transfer_claim_failed_not_owner() {
	new_test_ext().execute_with(|| {
		let claim = vec![0, 1];
		let _ = PoeModule::create_claim(Origin::signed(1), claim.clone());

		assert_noop!(
            PoeModule::transfer_claim(Origin::signed(2), claim.clone(), 3),
            Error::<Test>::NotClaimOwner
        );
	})
}

// 测试key超过最大长度
#[test]
fn claim_too_long() {
	new_test_ext().execute_with(|| {
		let claim = vec![1; i32::MAX.try_into().unwrap()];

		assert_noop!(
			PoeModule::create_claim(Origin::signed(1), claim.clone()),
			Error::<Test>::ClaimTooLong
		);
	})
}