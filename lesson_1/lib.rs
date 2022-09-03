#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

/// A module for proof of the existence
// pub use frame_system::pallet::*;
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    // use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    use sp_std::prelude::*;

    // 1. 配置(定义)接口
    #[pallet::config]
    pub trait Config: frame_system::Config {
        
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

         // 链上存储Hash值，长度不变；同时因为是常量，所以用#[pallet::constant]宏声明
        #[pallet::constant]
        type MaxClaimLength: Get<u32>;
    }

    // 2. 定义模块需要的结构体
    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]       // 这个宏生成包含所有存储项的trait
    pub struct Pallet<T>(_);

    // 3. 定义要使用的存储项
    // 存储单元
    #[pallet::storage]
    #[pallet::getter(fn proofs)]   
    pub type Proofs<T: Config> = StorageMap<
        _,
        Blake2_128Concat,                      
        BoundedVec<u8, T::MaxClaimLength>,      // 不能再用Vec<u8>,
        (T::AccountId, T::BlockNumber)        
    >;


    // 4. 定义事件，可以在交易执行过程中触发
    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        ClaimCreated(T::AccountId, Vec<u8>),
        ClaimRevoked(T::AccountId, Vec<u8>),
        ClaimTransfered(T::AccountId, Vec<u8>, T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// 存证已经存在，已经被创建
        ProofAlreadyExist,
        /// 存证不存在，无法撤销
        ClaimNotExist,
        /// 该存证是由另外一个用户创建，当前账户无权处理
        NotClaimOwner,

        // Key过长
        ClaimTooLong
    }

    // 定义保留函数（非必需）， 这里不需要保留函数（存证模块不需要任何保留函数），保留函数是指在区块的不同时机执行的函数
    // 模块定义里有一些特殊的函数可以在区块的某一个时间执行，这些特殊的函数定义在Hooks里面
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        // 这里没有想要定义的特殊函数，所以为空
    }

    // 5. 定义可调用函数（在Pallet结构体里添加）
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]    
        pub fn create_claim(origin: OriginFor<T>, claim: Vec<u8>) -> DispatchResultWithPostInfo {
            // 5.1 校验发送方，并且在校验完成后获取发送方的ID
            let sender = ensure_signed(origin)?;

            // 5.2 校验存证内容的Hash值是否超过最大长度
            // 把BoundedVec尝试转成Vec<u8>，如果失败，就报错
            let bounded_claim: BoundedVec<u8, <T as Config>::MaxClaimLength> = BoundedVec::<u8, T::MaxClaimLength>::try_from(claim.clone())
                .map_err(|_| Error::<T>::ClaimTooLong)?;

            // 5.3 如果不存在存证，就返回错误
            ensure!(!Proofs::<T>::contains_key(&bounded_claim), Error::<T>::ProofAlreadyExist);

            // 5.4 存储记录
            Proofs::<T>::insert(
                &bounded_claim,
                // 第一个元素是发送者（存证的Owner）,第二个元素是区块
                (sender.clone(),frame_system::Pallet::<T>::block_number()),
            );

            // 5.5 插入成功，触发事件
            Self::deposit_event(Event::ClaimCreated(sender, claim));

            Ok(().into())
        }

        // 吊销存证
        #[pallet::weight(0)]
        pub fn revoke_claim(origin: OriginFor<T>, claim: Vec<u8>) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            let bounded_claim: BoundedVec<u8, <T as Config>::MaxClaimLength> = BoundedVec::<u8, T::MaxClaimLength>::try_from(claim.clone())
                .map_err(|_| Error::<T>::ClaimTooLong)?;

            // 查看存证值是否存在，只有存在才能吊销
            let (owner, _) = Proofs::<T>::get(&bounded_claim).ok_or(Error::<T>::ClaimNotExist)?;

            // 只有Owner才可以吊销
            ensure!(owner == sender, Error::<T>::NotClaimOwner);

            Proofs::<T>::remove(&bounded_claim);

            Self::deposit_event(Event::ClaimRevoked(sender, claim));
            Ok(().into())
        }

        // 转移存证
        #[pallet::weight(0)]
        pub fn transfer_claim(origin: OriginFor<T>, claim: Vec<u8>, dest: T::AccountId) -> DispatchResultWithPostInfo {
            // 检查发送方是否合法
            let sender = ensure_signed(origin)?;

            let bounded_claim: BoundedVec<u8, <T as Config>::MaxClaimLength> = BoundedVec::<u8, T::MaxClaimLength>::try_from(claim.clone())
                .map_err(|_| Error::<T>::ClaimTooLong)?;

            // 检查存证是否存在
            ensure!(Proofs::<T>::contains_key(&bounded_claim), Error::<T>::ClaimNotExist);
            let (owner, _) = Proofs::<T>::get(&bounded_claim).unwrap();

            // 检查sender是否为owner
            ensure!(sender == owner, Error::<T>::NotClaimOwner);

            Proofs::<T>::insert(&bounded_claim, (dest, frame_system::Pallet::<T>::block_number()));

            Ok(().into())
        }
    }
}