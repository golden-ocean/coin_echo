//! 实体标识。
//!
//! # 为什么不用裸 `Uuid`
//!
//! `fn transfer(from: Uuid, to: Uuid, order: Uuid)` 这样的签名，把 `order` 传到
//! `from` 的位置编译器不会有任何反应，而这类 bug 在生产环境代价极高。
//!
//! [`Id<T>`] 通过幻影类型把实体种类编码进类型系统：`Id<User>` 与 `Id<Order>` 是
//! 两个不同的类型，传错直接编译失败。运行期表示仍是 16 字节的 `Uuid`，零额外开销。
//!
//! # 为什么用 `UUIDv7`
//!
//! v4 完全随机，作为主键会让 B-tree 索引的插入点均匀散布在整棵树上，造成频繁
//! 页分裂与缓存失效。v7 高位是毫秒时间戳，天然有序，插入集中在树的右端，同时
//! 保留了分布式生成无需协调的优点。
//!
//! # 为什么不用 `Option<Id<T>>` 之外的"空标识"表示
//!
//! 曾考虑提供 `nil()`/`is_nil()` 这样的哨兵值表示"无标识"，但这会让「没有值」
//! 有两条并行的表达路径（`Option::None` 和 `nil()`），调用方需要同时防两处，
//! 容易一处判断 `is_none()`、另一处判断 `is_nil()` 导致遗漏。统一用
//! `Option<Id<T>>` 表达缺省状态。

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// 实体类型标签。
///
/// 实现此 trait 的类型可作为 [`Id<T>`] 的标签，`ENTITY` 用于日志与「资源不存在」
/// 类错误的信息拼装。
///
/// 标签类型通常就是领域实体本身（`impl Entity for User`）；也可以是零大小的
/// 占位类型，用于给尚无实体结构的标识分类。
pub trait Entity {
    /// 实体的稳定名称，蛇形命名，如 `user`、`order_item`。
    const ENTITY: &'static str;
}

/// 类型安全的实体标识。
///
/// # 示例
///
/// ```
/// use platform_kernel::id::{Entity, Id};
///
/// struct User;
/// impl Entity for User {
///     const ENTITY: &'static str = "user";
/// }
///
/// let a: Id<User> = Id::generate();
/// let b: Id<User> = a; // Copy，无需 clone
/// assert_eq!(a, b);
/// ```
///
/// 幻影字段使用 `fn() -> T` 而非 `T`：这样无论 `T` 是否 `Send`/`Sync`，
/// `Id<T>` 都是 `Send + Sync`，且不会被视为持有 `T` 的所有权（不影响 drop 检查）。
///
/// `PartialEq`/`Eq`/`Ord`/`Hash` 等全部手动实现而非 `derive`：`derive` 宏会
/// 无条件给 `impl` 加上 `T: PartialEq` 之类的 bound，即使 `T` 只是通过
/// `PhantomData<fn() -> T>` 幻影持有、根本不参与比较。这会导致标签类型
/// （如未 derive `Eq` 的 `User`）无法让 `Id<User>` 参与 `==` 比较——这是
/// Rust 已知的 derive 限制（rust-lang/rust#26925），不是可以忽略的边缘情况。
pub struct Id<T: ?Sized> {
    value: Uuid,
    tag: PhantomData<fn() -> T>,
}

impl<T: ?Sized> Id<T> {
    /// 生成一个新的时间有序标识（UUIDv7）。
    ///
    /// 命名为 `generate` 而非 `new`：`new` 通常暗示确定性构造，而这个方法
    /// 每次调用都读系统时钟、产生不同的值，需要在调用点显式提醒这一点。
    /// 需要确定性标识的测试场景请改用 [`IdSource`] 注入。
    #[must_use]
    pub fn generate() -> Self {
        Self::from_uuid(Uuid::now_v7())
    }

    /// 由已有 `Uuid` 构造。用于从数据库行或外部输入还原标识。
    #[must_use]
    pub const fn from_uuid(value: Uuid) -> Self {
        Self {
            value,
            tag: PhantomData,
        }
    }

    /// 取出底层 `Uuid`，用于持久化与跨边界传输。
    ///
    /// `self` 按值传递而非 `&self`：`Id<T>` 是 `Copy` 类型，访问器按 Rust
    /// 惯例应直接消费值而非借用，避免多余的引用间接。
    #[must_use]
    pub const fn as_uuid(self) -> Uuid {
        self.value
    }

    /// 改变标签类型。
    ///
    /// 仅用于确实需要在标识种类间转换的场景（例如聚合根与其快照共用标识）。
    /// 这是一道刻意设置的门槛：调用点显式可见，评审时容易发现滥用。
    #[must_use]
    pub const fn retag<U: ?Sized>(self) -> Id<U> {
        Id::from_uuid(self.value)
    }
}

// ── 手动实现的 trait ────────────────────────────────────────────────────────
//
// 全部手动实现而非 derive：derive 会为标签类型 T 添加同名 bound（要求
// `T: Clone` 才能 `Id<T>: Clone`），但 T 只是编译期标签，运行期不存在，
// 不应对其提出任何要求。

impl<T: ?Sized> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for Id<T> {}

impl<T: ?Sized> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: ?Sized> Eq for Id<T> {}

impl<T: ?Sized> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: ?Sized> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T: ?Sized> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T: ?Sized> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id({})", self.value)
    }
}

impl<T: ?Sized> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl<T: ?Sized> From<Uuid> for Id<T> {
    fn from(value: Uuid) -> Self {
        Self::from_uuid(value)
    }
}

impl<T: ?Sized> From<Id<T>> for Uuid {
    fn from(id: Id<T>) -> Self {
        id.value
    }
}

impl<T: ?Sized> FromStr for Id<T> {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self::from_uuid)
    }
}

impl<T: ?Sized> Serialize for Id<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}

impl<'de, T: ?Sized> Deserialize<'de> for Id<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Uuid::deserialize(deserializer).map(Self::from_uuid)
    }
}

/// 标识生成端口。
///
/// 生产环境注入 [`Uuidv7Source`]；测试中注入固定序列的实现，即可断言生成的
/// 标识，无需在断言里绕开随机值。
///
/// 方法签名刻意使用 `Uuid` 而非泛型 `Id<T>`：泛型方法会让 trait 失去对象安全性，
/// 无法以 `Arc<dyn IdSource>` 形式注入。类型标签通过 [`IdSource::next_id`] 在
/// 调用点补上。
pub trait IdSource: fmt::Debug + Send + Sync + 'static {
    /// 产生下一个标识值。
    fn next_uuid(&self) -> Uuid;

    /// 产生带类型标签的标识。
    ///
    /// `where Self: Sized` 保证该默认方法不影响 trait 的对象安全性；
    /// 持有 `dyn IdSource` 时改用 `Id::from_uuid(src.next_uuid())`。
    fn next_id<T: ?Sized>(&self) -> Id<T>
    where
        Self: Sized,
    {
        Id::from_uuid(self.next_uuid())
    }
}

/// 基于系统时钟的 `UUIDv7` 生成器，生产环境默认实现。
#[derive(Debug, Clone, Copy, Default)]
pub struct Uuidv7Source;

impl IdSource for Uuidv7Source {
    fn next_uuid(&self) -> Uuid {
        Uuid::now_v7()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct User;
    impl Entity for User {
        const ENTITY: &'static str = "user";
    }

    struct Order;
    impl Entity for Order {
        const ENTITY: &'static str = "order";
    }

    #[test]
    fn v7_ids_are_time_ordered() {
        let mut prev = Id::<User>::generate();
        for _ in 0..64 {
            std::thread::sleep(std::time::Duration::from_millis(2));
            let next = Id::<User>::generate();
            assert!(next > prev, "UUIDv7 应保持时间有序");
            prev = next;
        }
    }

    #[test]
    fn round_trips_through_string_and_json() {
        let id = Id::<Order>::generate();
        let parsed: Id<Order> = id.to_string().parse().expect("解析自身输出不应失败");
        assert_eq!(id, parsed);

        let json = serde_json::to_string(&id).expect("序列化不应失败");
        let back: Id<Order> = serde_json::from_str(&json).expect("反序列化不应失败");
        assert_eq!(id, back);
    }

    #[test]
    fn source_is_object_safe() {
        let src: std::sync::Arc<dyn IdSource> = std::sync::Arc::new(Uuidv7Source);
        let a = Id::<User>::from_uuid(src.next_uuid());
        let b = Id::<User>::from_uuid(src.next_uuid());
        assert_ne!(a, b);
    }

    #[test]
    fn entity_name_is_available_for_messages() {
        assert_eq!(User::ENTITY, "user");
        assert_eq!(Order::ENTITY, "order");
    }

    /// 未 derive Eq/Ord/Hash 的标签类型仍能让 Id<T> 参与比较——
    /// 这条测试专门验证文档3版本会出问题的那个场景。
    #[test]
    fn id_comparable_even_when_tag_type_lacks_common_derives() {
        struct Bare; // 故意什么都不 derive

        let a: Id<Bare> = Id::from_uuid(Uuid::nil());
        let b: Id<Bare> = Id::from_uuid(Uuid::nil());
        assert_eq!(a, b); // 若 Id 用 derive 实现，这里在编译期就会失败
    }

    #[test]
    fn retag_preserves_uuid_value_across_types() {
        let user_id: Id<User> = Id::generate();
        let raw = user_id.as_uuid();
        let order_id: Id<Order> = user_id.retag();
        assert_eq!(order_id.as_uuid(), raw);
    }

    #[test]
    fn ordering_reflects_underlying_uuid_order() {
        let smaller = Id::<User>::from_uuid(Uuid::nil());
        let larger = Id::<User>::from_uuid(Uuid::from_u128(u128::MAX));
        assert!(smaller < larger);
    }

    #[test]
    fn from_str_rejects_invalid_uuid() {
        let result: Result<Id<User>, _> = "not-a-uuid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn uuid_conversions_round_trip() {
        let raw = Uuid::now_v7();
        let id: Id<User> = raw.into();
        let back: Uuid = id.into();
        assert_eq!(raw, back);
    }

    #[test]
    fn json_serializes_as_bare_uuid_string_not_wrapped_object() {
        let id = Id::<User>::from_uuid(Uuid::nil());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"00000000-0000-0000-0000-000000000000\"");
    }

    /// 验证 `PhantomData<fn() -> T>` 的设计目标：即使标签类型 T 本身不是
    /// Send/Sync（比如内部含 Rc），Id<T> 依然是 Send + Sync。这是选用
    /// `fn() -> T` 而非 `T` 作为幻影字段的唯一理由，必须有测试锁定。
    #[test]
    fn id_is_send_and_sync_regardless_of_tag_type() {
        fn assert_send_sync<X: Send + Sync>() {}
        struct NotSendSync(#[allow(dead_code)] std::rc::Rc<()>);
        assert_send_sync::<Id<NotSendSync>>();
    }
}
