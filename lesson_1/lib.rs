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
        // 在Runtime进行配置接口实现时，会把Runtime定义的Event设置在这个类型里
        // 我们要求的Event满足这些条件：从当前模块Event类型转换过去，同时是系统模块的Event类型
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        // 进阶版内容：
        /// The maximum length of claim that can be added.  （这句注释会在前端展示）
        // 链上存储Hash值，长度不变；同时因为是常量，所以用#[pallet::constant]宏声明
        #[pallet::constant]
        type MaxClaimLength: Get<u32>;
    }

    // 2. 定义模块需要的结构体
    // 承载功能模块的pallet
    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]       // 这个宏生成包含所有存储项的trait
    pub struct Pallet<T>(_);

    // 3. 定义要使用的存储项
    // 存储单元
    #[pallet::storage]
    #[pallet::getter(fn proofs)]    // 使用宏来定义了一个getter函数，叫proofs，即会触发被宏标注的代码
    // (T::AccountId, T::BlockNumber)，AccountId表示用户ID，BlockNumber表示存入存证的区块，这两个类型都来自系统模块
    // 进阶版：在新版本里，Runtime不能直接使用Vec<_>，而是要用BoundedVec<_>这样更安全的长度受限的集合类型
    pub type Proofs<T: Config> = StorageMap<
        _,
        Blake2_128Concat,                       // Blake2是密码学安全的Hash算法
        BoundedVec<u8, T::MaxClaimLength>,      // 不能再用Vec<u8>,
        (T::AccountId, T::BlockNumber)          // 第4个值是value，这里表示存证属于哪个用户，存在哪个区块
    >;


    // 4. 定义事件，可以在交易执行过程中触发
    // #[pallet::metadata(T::AccountId = "AccountId")]          // 转换给前端
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

        // 进阶版加上的
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
        // 创建存证的可调用函数
        // origin表示发送方，claim表示存证的Hash值
        #[pallet::weight(0)]    // 实际情况是weight必须先测试得到合理的值，并且weight的选取是和存储单元有直接关系
        pub fn create_claim(origin: OriginFor<T>, claim: Vec<u8>) -> DispatchResultWithPostInfo {
            // 5.1 校验发送方，并且在校验完成后获取发送方的ID
            let sender = ensure_signed(origin)?;

            // 进阶版新增
            // 5.2 校验存证内容的Hash值是否超过最大长度
            // 把BoundedVec尝试转成Vec<u8>，如果失败，就报错
            let bounded_claim: BoundedVec<u8, <T as Config>::MaxClaimLength> = BoundedVec::<u8, T::MaxClaimLength>::try_from(claim.clone())
                .map_err(|_| Error::<T>::ClaimTooLong)?;

            // 5.3 如果不存在存证，就返回错误
            // ensure!(!Proofs::<T>::contains_key(&claim), Error::<T>::ProofAlreadyExist);
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

            // 开始转移
            // let current_block = <frame_system::Pallet<T>>::block_number();
            // Proofs::<T>::mutate(&bounded_claim, |value| {
            //     value.as_mut().unwrap().0 = recv_account.clone();
            //     value.as_mut().unwrap().1 = current_block;
            // });
            //
            // Self::deposit_event(Event::ClaimTransfered(sender, claim, recv_account));

            Proofs::<T>::insert(&bounded_claim, (dest, frame_system::Pallet::<T>::block_number()));

            Ok(().into())
        }
    }
}