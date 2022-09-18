#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

// #[cfg(test)]
// mod mock;
//
// #[cfg(test)]
// mod tests;

#[frame_support::pallet]
pub mod pallet {
	// use frame_support::transactional;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_io::hashing::blake2_128;
	use frame_support::traits::{Randomness, Currency, ReservableCurrency};
	use sp_runtime::traits::{AtLeast32BitUnsigned, Bounded, One};

	// KittyIndex definition move to runtime, 但是不在这里定义的话，会报错？
	type KittyIndex = u32;

	// #[pallet::type_value]
	// pub fn GetDefaultValue() -> KittyIndex {
	// 	0_u32
	// }

	#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	pub struct Kitty(pub [u8; 16]);

	// 固定写法
	type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;

		// 定义KittyIndex类型: 在runtime中实现
		type KittyIndex: Parameter + Member + AtLeast32BitUnsigned  + Default + Copy + MaxEncodedLen + Bounded;

		type MaxLength: Get<u32>;

		// 创建Kitty需要质押token保留的数量
		type KittyReserve:Get<BalanceOf<Self>>;

		// Currency 类型，用于质押等于资产相关的操作
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn next_kitty_id)]
	pub type NextKittyId<T> = StorageValue<_, KittyIndex, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn kitties)]
	pub type Kitties<T> = StorageMap<_, Blake2_128Concat, KittyIndex, Kitty>;

	#[pallet::storage]
	#[pallet::getter(fn kitty_owner)]
	pub type KittyOwner<T: Config> = StorageMap<_, Blake2_128Concat, KittyIndex, T::AccountId>;

	// get all kitties
	#[pallet::storage]
	#[pallet::getter(fn all_owner_kitty)]
	pub type AllOwnerKitty<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<Kitty, T::MaxLength>, ValueQuery>;


	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		KittyCreated(T::AccountId, KittyIndex, Kitty),
		KittyBred(T::AccountId, KittyIndex, Kitty),
		KittyTransferred(T::AccountId, T::AccountId, KittyIndex),
		TokenStake(T::AccountId)
	}

	#[pallet::error]
	pub enum Error<T> {
		InvalidKittyId,
		NotOwner,
		SameKittyId,
		ExceedMaxKittyOwned,
		TokenNotEnough,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn create(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let kitty_id = Self::get_next_id().map_err(|_| Error::<T>::InvalidKittyId)?;

			// 创建新Kitty，质押token
			Self::stake(&who)?;

			let dna = Self::random_value(&who);
			let kitty = Kitty(dna);

			Kitties::<T>::insert(kitty_id, &kitty);
			KittyOwner::<T>::insert(kitty_id, &who);
			NextKittyId::<T>::set(kitty_id + 1);

			// AllOwnerKitty::<T>::try_mutate(&who, |kitty_vec| {
			// 	kitty_vec.try_push(kitty.clone())
			// }).map_err(|_| <Error<T>>::ExceedMaxKittyOwned)?;

			Self::add_kitty_to_map(&who, &kitty);

			// Emit an event.
			Self::deposit_event(Event::KittyCreated(who, kitty_id, kitty));
			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn breed(origin: OriginFor<T>, kitty_id_1: KittyIndex, kitty_id_2: KittyIndex) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// check kitty id,父母不能是同一个kitty
			ensure!(kitty_id_1 != kitty_id_2, Error::<T>::SameKittyId);

			let kitty_1 = Self::get_kitty(kitty_id_1).map_err(|_| Error::<T>::InvalidKittyId)?;
			let kitty_2 = Self::get_kitty(kitty_id_2).map_err(|_| Error::<T>::InvalidKittyId)?;

			// get next id
			let kitty_id = Self::get_next_id().map_err(|_| Error::<T>::InvalidKittyId)?;

			// 繁殖kitty需要质押token
			Self::stake(&who);

			// selector for breeding
			let selector = Self::random_value(&who);

			let mut data = [0u8; 16];
			for i in 0..kitty_1.0.len() {
				// 0 choose kitty2, and 1 choose kitty1
				data[i] = (kitty_1.0[i] & selector[i]) | (kitty_2.0[i] & !selector[i]);
			}
			let new_kitty = Kitty(data);

			<Kitties<T>>::insert(kitty_id, &new_kitty);
			KittyOwner::<T>::insert(kitty_id, &who);
			NextKittyId::<T>::set(kitty_id + 1);

			// 繁殖kitty时，需要增加到扩展存储项中
			Self::add_kitty_to_map(&who, &new_kitty);

			Self::deposit_event(Event::KittyCreated(who, kitty_id, new_kitty));

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn transfer(origin: OriginFor<T>, kitty_id: u32, new_owner: T::AccountId) -> DispatchResult {
			let prev_owner = ensure_signed(origin)?;

			let exsit_kitty = Self::get_kitty(kitty_id).map_err(|_| Error::<T>::InvalidKittyId)?;

			// 在map中查询，然后检查是否为kitty的owner
			ensure!(Self::kitty_owner(kitty_id) == Some(prev_owner.clone()), Error::<T>::NotOwner);

			// 新拥有者质押token
			// T::Currency::reserve(&new_owner, T::KittyReserve::get()).map_err(|_| Error::<T>::TokenNotEnough)?;
			Self::stake(&new_owner);

			// 删除原拥有者AllOwnerKitty存储项需转移的kitty
			AllOwnerKitty::<T>::try_mutate(&prev_owner, |owned| {
				if let Some(index) = owned.iter().position(|kitty| kitty == &exsit_kitty) {
					owned.swap_remove(index);
					return Ok(());
				}
				Err(())
			}).map_err(|_| <Error<T>>::NotOwner)?;

			// 解押原来拥有都质押的token
			T::Currency::unreserve(&prev_owner, T::KittyReserve::get());

			<KittyOwner<T>>::insert(kitty_id, &new_owner);

			// 追加转移的kitty到新拥有者AllOwnerKitty存储项中
			AllOwnerKitty::<T>::try_mutate(&new_owner, |vec| {
				vec.try_push(exsit_kitty)
			}).map_err(|_| <Error<T>>::ExceedMaxKittyOwned)?;

			Self::deposit_event(Event::KittyTransferred(prev_owner,new_owner,kitty_id));

			Ok(())
		}

	}

	impl<T: Config> Pallet<T> {
		// get a random 256.
		fn random_value(sender: &T::AccountId) -> [u8; 16] {

			let payload = (
				T::Randomness::random_seed(),
				&sender,
				<frame_system::Pallet::<T>>::extrinsic_index(),
			);

			payload.using_encoded(blake2_128)
		}

		// get netx id
		fn get_next_id() -> Result<KittyIndex, ()> {
			match Self::next_kitty_id() {
				KittyIndex::MAX => Err(()),
				val => Ok(val),
			}
		}

		// get kitty via id
		fn get_kitty(kitty_id: KittyIndex) -> Result<Kitty, ()> {
			match Self::kitties(kitty_id) {
				Some(kitty) => Ok(kitty),
				None => Err(()),
			}
		}

		// 质押token
		// 这个函数如果要放在impl<T: Config> Pallet<T>的话，第一个参数是OriginFor<T>
		// 所以只能放在这个位置
		pub fn stake(who: &T::AccountId) -> DispatchResult {

			T::Currency::reserve(who, T::KittyReserve::get()).map_err(|_| Error::<T>::TokenNotEnough)?;

			Self::deposit_event(Event::TokenStake(who.clone()));

			Ok(())
		}

		// 繁殖kitty时，需要增加到扩展存储项中
		pub fn add_kitty_to_map(who: &T::AccountId, kitty: &Kitty) -> DispatchResultWithPostInfo {
			AllOwnerKitty::<T>::try_mutate(who, |kitty_vec| {
				kitty_vec.try_push(kitty.clone())
			}).map_err(|_| <Error<T>>::ExceedMaxKittyOwned)?;

			Ok(().into())
		}
	}
}
