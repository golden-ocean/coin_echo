use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use platform_kernel::meta::{AuditMeta, DeleteMeta, Status, VersionMeta};

use crate::{
    error::DomainError,
    id::{OrganizationId, PositionId, RoleId, UserId},
    user::value_object::{
        DataScope, Email, EmploymentStatus, Gender, PasswordCredential, Phone, StaffNo,
    },
};

/// 用户领域聚合根
/// 封装用户全量业务状态、身份凭证、权限范围、人事信息、审计软删除、乐观锁版本
/// 业务约束：
/// 1. 内置系统用户 is_builtin = true 禁止修改、禁用、删除，受系统保护
/// 2. 已软删除用户无法执行任何修改操作
/// 3. 账号禁用状态下，个人资料、自主改密拦截；找回密码/管理员重置密码不受禁用限制
/// 4. 自主改密存在24小时冷却期，管理员重置密码无冷却限制
#[derive(Debug)]
pub struct User {
    id: UserId,
    username: String,
    staff_no: StaffNo,
    name: String,
    email: Email,
    phone: Phone,
    gender: Gender,
    birthday: Option<NaiveDate>,
    avatar: Option<String>,
    password_credential: PasswordCredential,

    employment_status: EmploymentStatus,
    data_scope: DataScope,
    is_builtin: bool,
    sort: i32,
    remark: Option<String>,
    status: Status,

    organization_id: Option<OrganizationId>,
    position_id: Option<PositionId>,
    role_ids: Vec<RoleId>,

    audit_meta: AuditMeta,
    delete_meta: DeleteMeta,
    version_meta: VersionMeta,
}

impl User {
    /// 业务新建用户工厂方法
    /// 用于注册、管理员新增用户，填充默认初始值
    /// 默认：性别未知、无生日头像、在职、仅本人数据权限、启用、无岗位角色、版本0
    pub fn new(
        id: UserId,
        username: String,
        password_credential: PasswordCredential,
        staff_no: StaffNo,
        name: String,
        email: Email,
        phone: Phone,
        organization_id: Option<OrganizationId>,
        operator_id: Option<Uuid>,
        sort: Option<i32>,
        status: Option<Status>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            username,
            staff_no,
            name,
            email,
            phone,
            gender: Gender::Unknown,
            birthday: None,
            avatar: None,
            password_credential,
            employment_status: EmploymentStatus::Active,
            data_scope: DataScope::SelfOnly,
            is_builtin: false,
            sort: sort.unwrap_or(1000),
            remark: None,
            status: status.unwrap_or(Status::Enabled),
            organization_id,
            position_id: None,
            role_ids: vec![],
            audit_meta: AuditMeta::new(operator_id, now),
            delete_meta: DeleteMeta::new(),
            version_meta: VersionMeta::new(),
        }
    }

    /// 持久层还原工厂方法
    /// 仅仓库层使用，数据库查询完整字段后重建聚合根，不做业务默认填充
    pub fn restore(
        id: UserId,
        username: String,
        staff_no: StaffNo,
        name: String,
        email: Email,
        phone: Phone,
        gender: Gender,
        birthday: Option<NaiveDate>,
        avatar: Option<String>,
        password_credential: PasswordCredential,
        employment_status: EmploymentStatus,
        data_scope: DataScope,
        is_builtin: bool,
        sort: i32,
        remark: Option<String>,
        status: Status,
        organization_id: Option<OrganizationId>,
        position_id: Option<PositionId>,
        role_ids: Vec<RoleId>,
        audit_meta: AuditMeta,
        delete_meta: DeleteMeta,
        version_meta: VersionMeta,
    ) -> Self {
        Self {
            id,
            username,
            staff_no,
            name,
            email,
            phone,
            gender,
            birthday,
            avatar,
            password_credential,
            employment_status,
            data_scope,
            is_builtin,
            sort,
            remark,
            status,
            organization_id,
            position_id,
            role_ids,
            audit_meta,
            delete_meta,
            version_meta,
        }
    }
    /// 通用修改前置校验：内置账号、已删除拦截
    pub fn ensure_modifiable(&self) -> Result<(), DomainError> {
        if self.is_builtin {
            return Err(DomainError::UserProtected { id: self.id });
        }
        if self.delete_meta.is_deleted() {
            return Err(DomainError::UserNotFound { id: self.id });
        }
        Ok(())
    }

    /// 校验是否可执行"需账号启用"的操作:资料编辑、自主改密等
    /// 内置账号、已删除、已禁用账号均拦截
    fn ensure_self_modifiable(&self) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        if self.status.is_disabled() {
            return Err(DomainError::UserSuspended { id: self.id });
        }
        Ok(())
    }

    /// 更新基础个人资料：姓名、邮箱、手机号
    /// 拦截内置账号、已删除、已禁用账号
    pub fn update_info(
        &mut self,
        new_name: String,
        new_email: Email,
        new_phone: Phone,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_self_modifiable()?;
        self.name = new_name;
        self.email = new_email;
        self.phone = new_phone;
        self.audit_meta.update(operator_id, now);
        self.version_meta = self.version_meta.next();
        Ok(())
    }

    pub fn delete(
        &mut self,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;

        self.audit_meta.update(operator_id, now);
        self.delete_meta.delete(operator_id, now);
        self.version_meta = self.version_meta.next();

        Ok(())
    }

    /// 用户自主修改密码（个人中心）
    /// 内置/删除/禁用账号禁止操作
    /// 受冷却期限制，防止盗号后被连续篡改
    pub fn change_password(
        &mut self,
        new_password_hash: String,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_self_modifiable()?;
        self.password_credential = self
            .password_credential
            .change(new_password_hash.as_str(), now)?;
        self.audit_meta.update(operator_id, now);
        self.version_meta = self.version_meta.next();
        Ok(())
    }

    /// 管理员/找回密码强制重置密码
    /// 不受冷却期限制；账号禁用仍可重置（被盗恢复场景）
    /// 内置账号禁止重置保护系统
    pub fn reset_password(
        &mut self,
        new_password_hash: String,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        self.password_credential = self
            .password_credential
            .reset(new_password_hash.as_str(), now)?;
        self.audit_meta.update(operator_id, now);
        self.version_meta = self.version_meta.next();
        Ok(())
    }

    /// 判断密码是否超过最大有效期
    pub fn is_password_expired(&self, max_age_days: i64, now: DateTime<Utc>) -> bool {
        self.password_credential.is_expired(max_age_days, now)
    }

    /// 校验登录Token是否有效：Token签发时间晚于最后一次改密则有效
    /// 密码修改后所有旧Token自动失效
    pub fn is_token_valid_against_password_change(&self, token_issued_at: DateTime<Utc>) -> bool {
        token_issued_at >= self.password_credential.updated_at()
    }

    /// 禁用用户账号
    /// 内置系统账号不可禁用禁用账号
    pub fn disable(&mut self, operator_id: Uuid, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        if self.status == Status::Disabled {
            return Err(DomainError::UserStatusAlreadyDisabled { id: self.id });
        }
        self.status = Status::Disabled;
        self.audit_meta.update(Some(operator_id), now);
        self.version_meta = self.version_meta.next();
        Ok(())
    }

    /// 启用账号
    pub fn enable(&mut self, operator_id: Uuid, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        if self.status == Status::Enabled {
            return Err(DomainError::UserStatusAlreadyEnabled { id: self.id });
        }
        self.status = Status::Enabled;
        self.audit_meta.update(Some(operator_id), now);
        self.version_meta = self.version_meta.next();
        Ok(())
    }

    // ===================== 业务辅助判断方法 =====================
    /// 是否已软删除
    pub fn is_deleted(&self) -> bool {
        self.delete_meta.is_deleted()
    }

    /// 是否正常可用：未删除 + 启用 + 在职
    pub fn is_normal_active(&self) -> bool {
        !self.delete_meta.is_deleted()
            && self.status.is_enabled()
            && self.employment_status.is_still_employed()
    }

    // ===================== 字段只读Getter =====================
    pub fn id(&self) -> &UserId {
        &self.id
    }
    pub fn username(&self) -> &str {
        &self.username
    }
    pub fn staff_no(&self) -> &StaffNo {
        &self.staff_no
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn email(&self) -> &Email {
        &self.email
    }
    pub fn phone(&self) -> &Phone {
        &self.phone
    }
    pub fn gender(&self) -> Gender {
        self.gender
    }
    pub fn birthday(&self) -> Option<NaiveDate> {
        self.birthday
    }
    pub fn avatar(&self) -> Option<&str> {
        self.avatar.as_deref()
    }
    pub fn password_credential(&self) -> &PasswordCredential {
        &self.password_credential
    }
    pub fn employment_status(&self) -> EmploymentStatus {
        self.employment_status
    }
    pub fn data_scope(&self) -> DataScope {
        self.data_scope
    }
    pub fn is_builtin(&self) -> bool {
        self.is_builtin
    }
    pub fn sort(&self) -> i32 {
        self.sort
    }
    pub fn remark(&self) -> Option<&str> {
        self.remark.as_deref()
    }
    pub fn status(&self) -> Status {
        self.status
    }
    pub fn organization_id(&self) -> Option<&OrganizationId> {
        self.organization_id.as_ref()
    }
    pub fn position_id(&self) -> Option<&PositionId> {
        self.position_id.as_ref()
    }
    pub fn role_ids(&self) -> &[RoleId] {
        &self.role_ids
    }
    pub fn audit_meta(&self) -> &AuditMeta {
        &self.audit_meta
    }
    pub fn delete_meta(&self) -> &DeleteMeta {
        &self.delete_meta
    }
    pub fn version_meta(&self) -> &VersionMeta {
        &self.version_meta
    }
}

#[cfg(test)]
mod user_aggregate_tests {
    use super::*;
    use crate::user::value_object::{
        Email, PasswordCredential, PasswordCredentialError, Phone, StaffNo,
    };
    use chrono::{TimeDelta, TimeZone, Utc};
    use uuid::Uuid;

    /// 全局基准测试时间：2026-01-01 10:00:00
    fn base_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap()
    }

    /// 生成合法测试PHC密码哈希
    fn test_phc_hash() -> &'static str {
        "$argon2id$v=19,m=4096,t=3,p=2$testsalt$testhashvalue123456"
    }

    /// 快速构造标准正常用户（非内置、未删除、启用、在职）
    fn build_normal_test_user(operator: Option<Uuid>, now: DateTime<Utc>) -> User {
        let uid = UserId::generate();
        let pwd = PasswordCredential::new(test_phc_hash(), now).unwrap();
        let staff_no = StaffNo::new("STAFF-000001").unwrap();
        let email = Email::new("test@company.com").unwrap();
        let phone = Phone::new("13800138000").unwrap();

        User::new(
            uid,
            "test_user".to_string(),
            pwd,
            staff_no,
            "测试用户".to_string(),
            email,
            phone,
            None,
            operator,
            None,
            None,
            now,
        )
    }

    /// 生成内置系统管理员用户（is_builtin = true）
    fn build_builtin_admin(operator: Option<Uuid>, now: DateTime<Utc>) -> User {
        let mut user = build_normal_test_user(operator, now);
        // 强制修改为内置账号
        user.is_builtin = true;
        user
    }

    /// 生成已软删除用户
    fn build_deleted_user(operator: Option<Uuid>, now: DateTime<Utc>) -> User {
        let mut user = build_normal_test_user(operator, now);
        // 标记软删除
        user.delete_meta.delete(Some(Uuid::now_v7()), now);
        user
    }

    // ===================== 1. 工厂方法 new / restore 测试 =====================
    #[test]
    fn test_factory_new_default_fill() {
        let now = base_now();
        let op_id = UserId::generate();
        let user = build_normal_test_user(Some(op_id.as_uuid()), now);

        // 校验新建用户默认填充值
        assert_eq!(user.gender(), Gender::Unknown);
        assert!(user.birthday().is_none());
        assert!(user.avatar().is_none());
        assert!(user.position_id().is_none());
        assert!(user.role_ids().is_empty());
        assert!(user.remark().is_none());

        assert_eq!(user.employment_status(), EmploymentStatus::Active);
        assert_eq!(user.data_scope(), DataScope::SelfOnly);
        assert_eq!(user.status(), Status::Enabled);
        assert_eq!(user.sort(), 1000);
        assert!(!user.is_builtin());

        // 版本初始0
        assert_eq!(user.version_meta().value().to_string(), "0");
        // 审计创建人正确
        assert_eq!(user.audit_meta().created_by(), Some(op_id.as_uuid()));
    }

    #[test]
    fn test_factory_restore_full_recover() {
        let now = base_now();
        let mut origin = build_normal_test_user(None, now);
        // 修改全部可选字段，制造差异化
        origin.gender = Gender::Female;
        origin.birthday = Some(chrono::NaiveDate::from_ymd_opt(1990, 1, 1).unwrap());
        origin.avatar = Some("https://xxx.com/avatar.png".to_string());
        origin.position_id = Some(PositionId::generate());
        origin.role_ids = vec![RoleId::generate()];
        origin.remark = Some("测试备注".to_string());
        origin.data_scope = DataScope::DepartmentAndChildren;
        origin.employment_status = EmploymentStatus::OnLeave;
        origin.sort = 500;
        origin.is_builtin = true;

        // restore 完整还原所有字段，不会覆盖业务默认值
        let restore = User::restore(
            origin.id().clone(),
            origin.username().to_string(),
            origin.staff_no().clone(),
            origin.name().to_string(),
            origin.email().clone(),
            origin.phone().clone(),
            origin.gender(),
            origin.birthday(),
            origin.avatar().map(|v| v.to_string()),
            origin.password_credential().clone(),
            origin.employment_status(),
            origin.data_scope(),
            origin.is_builtin(),
            origin.sort(),
            origin.remark().map(|v| v.to_string()),
            origin.status(),
            origin.organization_id().cloned(),
            origin.position_id().cloned(),
            origin.role_ids.clone(),
            origin.audit_meta().clone(),
            origin.delete_meta().clone(),
            origin.version_meta().clone(),
        );

        // 全部字段完全一致
        assert_eq!(restore.gender(), origin.gender());
        assert_eq!(restore.birthday(), origin.birthday());
        assert_eq!(restore.avatar(), origin.avatar());
        assert_eq!(restore.position_id(), origin.position_id());
        // 切片转Vec匹配类型
        assert_eq!(restore.role_ids(), origin.role_ids().to_vec());
        assert_eq!(restore.data_scope(), origin.data_scope());
        assert_eq!(restore.employment_status(), origin.employment_status());
        assert_eq!(restore.is_builtin(), origin.is_builtin());
        assert_eq!(restore.sort(), origin.sort());
    }

    // ===================== 2. 公共前置校验 verify_can_modify 拦截规则 =====================
    #[test]
    fn test_verify_modify_builtin_protect() {
        let now = base_now();
        let admin = build_builtin_admin(None, now);
        // 内置账号禁止修改
        let res = admin.ensure_modifiable();
        assert!(matches!(res, Err(DomainError::UserProtected { .. })));
    }

    #[test]
    fn test_verify_modify_deleted_user() {
        let now = base_now();
        let deleted = build_deleted_user(None, now);
        // 已软删除用户拦截
        let res = deleted.ensure_modifiable();
        assert!(matches!(res, Err(DomainError::UserNotFound { .. })));
    }

    #[test]
    fn test_verify_modify_normal_pass() {
        let now = base_now();
        let normal = build_normal_test_user(None, now);
        // 正常用户校验通过
        assert!(normal.ensure_modifiable().is_ok());
    }

    // ===================== 3. 更新个人资料 update_profile =====================
    #[test]
    fn test_update_profile_normal_success() {
        let now = base_now();
        let op_uuid = Uuid::now_v7();
        let mut user = build_normal_test_user(None, now);

        let new_name = "新姓名".to_string();
        let new_email = Email::new("new@company.com").unwrap();
        let new_phone = Phone::new("13900139000").unwrap();

        user.update_info(
            new_name.clone(),
            new_email.clone(),
            new_phone.clone(),
            Some(op_uuid),
            now,
        )
        .unwrap();

        // 字段更新生效
        assert_eq!(user.name(), new_name);
        assert_eq!(user.email(), &new_email);
        assert_eq!(user.phone(), &new_phone);
        // 修复：Option<DateTime> 解包取值对比
        assert_eq!(user.audit_meta().updated_at(), now);
        assert_eq!(user.audit_meta().updated_by(), Some(op_uuid));
    }

    #[test]
    fn test_update_profile_suspended_block() {
        let now = base_now();
        let op_uuid = Uuid::now_v7();
        let mut user = build_normal_test_user(None, now);
        // 手动禁用账号
        user.status = Status::Disabled;

        let res = user.update_info(
            "xxx".into(),
            Email::new("a@b.com").unwrap(),
            Phone::new("13800138000").unwrap(),
            Some(op_uuid),
            now,
        );
        assert!(matches!(res, Err(DomainError::UserSuspended { .. })));
    }

    #[test]
    fn test_update_profile_builtin_deleted_block() {
        let now = base_now();
        let op_uuid = Uuid::now_v7();
        // 内置账号
        let mut admin = build_builtin_admin(None, now);
        let res1 = admin.update_info(
            "xxx".into(),
            Email::new("a@b.com").unwrap(),
            Phone::new("13800138000").unwrap(),
            Some(op_uuid),
            now,
        );
        assert!(matches!(res1, Err(DomainError::UserProtected { .. })));

        // 已删除账号
        let mut del = build_deleted_user(None, now);
        let res2 = del.update_info(
            "xxx".into(),
            Email::new("a@b.com").unwrap(),
            Phone::new("13800138000").unwrap(),
            Some(op_uuid),
            now,
        );
        assert!(matches!(res2, Err(DomainError::UserNotFound { .. })));
    }

    #[test]
    fn test_update_profile_bumps_version() {
        let now = base_now();
        let mut user = build_normal_test_user(None, now);
        let old_version = user.version_meta().value();

        user.update_info(
            "新姓名".into(),
            Email::new("new@company.com").unwrap(),
            Phone::new("13900139000").unwrap(),
            Some(Uuid::now_v7()),
            now,
        )
        .unwrap();

        assert_eq!(user.version_meta().value(), old_version + 1);
    }

    // ===================== 4. 自主改密 change_password（带24h冷却） =====================
    #[test]
    fn test_change_password_cooling_limit_block() {
        let now = base_now();
        let op_uuid = Uuid::now_v7();
        let mut user = build_normal_test_user(None, now);
        // 新 hash 必须与初始 hash 不同，否则命中 SameAsCurrent 校验
        let new_hash = "$argon2id$v=19,m=4096,t=3,p=2$newsalt$newhashvalue123456".to_string();

        // 仅间隔10小时，未到24h冷却期，拦截
        let short_time = now + TimeDelta::try_hours(10).unwrap();
        let err = user
            .change_password(new_hash.clone(), Some(op_uuid), short_time)
            .unwrap_err();
        assert!(matches!(
            err,
            DomainError::UserPasswordCredential(PasswordCredentialError::CoolingPeriodPassword)
        ));

        // 间隔25小时，冷却期结束允许修改
        let ok_time = now + TimeDelta::try_hours(25).unwrap();
        user.change_password(new_hash, Some(op_uuid), ok_time)
            .unwrap();
        // 密码更新时间刷新
        assert_eq!(user.password_credential().updated_at(), ok_time);
    }

    #[test]
    fn test_change_password_suspended_block() {
        let now = base_now();
        let op_uuid = Uuid::now_v7();
        let mut user = build_normal_test_user(None, now);
        user.status = Status::Disabled;

        let res = user.change_password(test_phc_hash().to_string(), Some(op_uuid), now);
        assert!(matches!(res, Err(DomainError::UserSuspended { .. })));
    }

    #[test]
    fn test_change_password_cooling_block_does_not_bump_version() {
        let now = base_now();
        let mut user = build_normal_test_user(None, now);
        let old_version = user.version_meta().value();

        // 冷却期内会失败
        let _ = user.change_password(
            test_phc_hash().to_string(),
            Some(Uuid::now_v7()),
            now + TimeDelta::try_hours(1).unwrap(),
        );

        // 关键断言：失败时 version 不应该被误增
        assert_eq!(user.version_meta().value(), old_version);
    }

    // ===================== 5. 管理员重置密码 reset_password（无视冷却、不禁用拦截） =====================
    #[test]
    fn test_reset_password_ignore_cooling_and_suspend() {
        let now = base_now();
        let op_uuid = Uuid::now_v7();
        let mut user = build_normal_test_user(None, now);
        // 禁用账号、仅间隔1小时，依然可以重置
        user.status = Status::Disabled;
        let reset_time = now + TimeDelta::try_hours(1).unwrap();
        let new_hash = "$argon2id$v=19,m=4096,t=3,p=2$salt$newhash".to_string();

        user.reset_password(new_hash.clone(), Some(op_uuid), reset_time)
            .unwrap();
        assert_eq!(user.password_credential().hash_as_str(), new_hash.as_str());
        assert_eq!(user.password_credential().updated_at(), reset_time);
    }

    #[test]
    fn test_reset_password_builtin_block() {
        let now = base_now();
        let op_uuid = Uuid::now_v7();
        let mut admin = build_builtin_admin(None, now);
        let res = admin.reset_password(test_phc_hash().to_string(), Some(op_uuid), now);
        assert!(matches!(res, Err(DomainError::UserProtected { .. })));
    }

    // ===================== 6. 启用/禁用账号 enable / disable =====================
    #[test]
    fn test_disable_enable_normal() {
        let now = base_now();
        let op_uuid = Uuid::now_v7();
        let mut user = build_normal_test_user(None, now);

        // 禁用
        user.disable(op_uuid, now).unwrap();
        assert_eq!(user.status(), Status::Disabled);

        // 启用
        user.enable(op_uuid, now).unwrap();
        assert_eq!(user.status(), Status::Enabled);
    }

    #[test]
    fn test_disable_enable_builtin_deleted_block() {
        let now = base_now();
        let op_uuid = Uuid::now_v7();
        // 内置账号禁用拦截
        let mut admin = build_builtin_admin(None, now);
        let res1 = admin.disable(op_uuid, now);
        assert!(matches!(res1, Err(DomainError::UserProtected { .. })));

        // 已删除账号禁用拦截
        let mut del = build_deleted_user(None, now);
        let res2 = del.disable(op_uuid, now);
        assert!(matches!(res2, Err(DomainError::UserNotFound { .. })));
    }

    // ===================== 7. 业务辅助判断方法 =====================
    #[test]
    fn test_is_normal_active_combine_rule() {
        let now = base_now();
        // 标准正常：未删+启用+在职
        let u1 = build_normal_test_user(None, now);
        assert!(u1.is_normal_active());

        // 已删除 → false
        let mut u2 = build_normal_test_user(None, now);
        u2.delete_meta.delete(None::<Uuid>, now);
        assert!(!u2.is_normal_active());

        // 禁用 → false
        let mut u3 = build_normal_test_user(None, now);
        u3.status = Status::Disabled;
        assert!(!u3.is_normal_active());

        // 离职 → false
        let mut u4 = build_normal_test_user(None, now);
        u4.employment_status = EmploymentStatus::Resigned;
        assert!(!u4.is_normal_active());

        // 休假保留岗位 → true
        let mut u5 = build_normal_test_user(None, now);
        u5.employment_status = EmploymentStatus::OnLeave;
        assert!(u5.is_normal_active());
    }

    #[test]
    fn test_is_token_valid_after_password_change() {
        let now = base_now();
        let mut user = build_normal_test_user(None, now);
        let change_time = now + TimeDelta::try_hours(5).unwrap();
        let token_early = now + TimeDelta::try_hours(2).unwrap();
        let token_late = now + TimeDelta::try_hours(8).unwrap();

        // 修改密码
        user.reset_password(test_phc_hash().to_string(), None, change_time)
            .unwrap();

        // Token 签发在改密前 → 失效
        assert!(!user.is_token_valid_against_password_change(token_early));
        // Token 签发在改密后 → 有效
        assert!(user.is_token_valid_against_password_change(token_late));
    }

    #[test]
    fn test_password_expired_clock_skew_safe() {
        let create = base_now();
        let user = build_normal_test_user(None, create);
        // 时钟回拨，当前时间早于创建时间，判定未过期
        let skew_time = create - TimeDelta::try_days(5).unwrap();
        assert!(!user.is_password_expired(30, skew_time));

        // 超过30天，判定过期
        let expire_time = create + TimeDelta::try_days(31).unwrap();
        assert!(user.is_password_expired(30, expire_time));
    }

    #[test]
    fn test_is_deleted_flag() {
        let now = base_now();
        let mut user = build_normal_test_user(None, now);
        assert!(!user.is_deleted());

        user.delete_meta.delete(None::<Uuid>, now);
        assert!(user.is_deleted());
    }

    // ===================== 8. Debug 脱敏安全校验 =====================
    #[test]
    fn test_debug_mask_sensitive_field_no_plain() {
        let now = base_now();
        let user = build_normal_test_user(None, now);
        let debug_text = format!("{:?}", user);

        // 明文邮箱、手机号、密码哈希不出现在Debug输出
        let raw_email = user.email().as_str();
        let raw_phone = user.phone().as_str();
        let raw_hash = user.password_credential().hash_as_str();

        assert!(!debug_text.contains(raw_email));
        assert!(!debug_text.contains(raw_phone));
        assert!(!debug_text.contains(raw_hash));
    }

    // ===================== 9. Getter 读取一致性校验 =====================
    #[test]
    fn test_all_getter_match_inner_field() {
        let now = base_now();
        let user = build_normal_test_user(None, now);

        assert_eq!(user.id().as_uuid(), user.id.as_uuid());
        assert_eq!(user.username(), &user.username);
        assert_eq!(user.staff_no(), &user.staff_no);
        assert_eq!(user.name(), &user.name);
        assert_eq!(user.email(), &user.email);
        assert_eq!(user.phone(), &user.phone);
        assert_eq!(user.gender(), user.gender);
        assert_eq!(user.birthday(), user.birthday);
        assert_eq!(user.avatar(), user.avatar.as_deref());
        assert_eq!(user.password_credential(), &user.password_credential);
        assert_eq!(user.employment_status(), user.employment_status);
        assert_eq!(user.data_scope(), user.data_scope);
        assert_eq!(user.is_builtin(), user.is_builtin);
        assert_eq!(user.sort(), user.sort);
        assert_eq!(user.remark(), user.remark.as_deref());
        assert_eq!(user.status(), user.status);
        assert_eq!(user.organization_id(), user.organization_id.as_ref());
        assert_eq!(user.position_id(), user.position_id.as_ref());
        // &[RoleId] 转 Vec 对比
        assert_eq!(user.role_ids().to_vec(), user.role_ids);
        assert_eq!(user.audit_meta(), &user.audit_meta);
        assert_eq!(user.delete_meta(), &user.delete_meta);
        assert_eq!(user.version_meta(), &user.version_meta);
    }

    // ===================== 10. 软删除 delete 测试 =====================

    /// 正常用户软删除成功
    #[test]
    fn test_delete_normal_success() {
        let now = base_now();
        let op_uuid = Uuid::now_v7();
        let mut user = build_normal_test_user(None, now);

        user.delete(Some(op_uuid), now).unwrap();

        assert!(user.is_deleted());
        assert_eq!(user.delete_meta().deleted_by(), Some(op_uuid));
        assert_eq!(user.delete_meta().deleted_at(), Some(now));
        assert_eq!(user.version_meta().value(), 1);
    }

    /// 已禁用用户仍可被删除（管理员操作不受启停限制）
    #[test]
    fn test_delete_disabled_user_allowed() {
        let now = base_now();
        let op_uuid = Uuid::now_v7();
        let mut user = build_normal_test_user(None, now);
        user.status = Status::Disabled;

        // 关键断言：禁用账号可以被删除
        user.delete(Some(op_uuid), now).unwrap();
        assert!(user.is_deleted());
    }

    /// 内置用户禁止删除
    #[test]
    fn test_delete_builtin_reject() {
        let now = base_now();
        let op_uuid = Uuid::now_v7();
        let mut admin = build_builtin_admin(None, now);

        let err = admin.delete(Some(op_uuid), now).unwrap_err();
        assert!(matches!(err, DomainError::UserProtected { .. }));
        assert!(!admin.is_deleted());
    }

    /// 已删除用户再次删除返回 NotFound
    #[test]
    fn test_delete_already_deleted_reject() {
        let now = base_now();
        let op_uuid = Uuid::now_v7();
        let mut user = build_deleted_user(None, now);

        let err = user.delete(Some(op_uuid), now).unwrap_err();
        assert!(matches!(err, DomainError::UserNotFound { .. }));
    }

    /// 删除失败时版本号不应递增
    #[test]
    fn test_delete_fail_no_version_bump() {
        let now = base_now();
        let mut admin = build_builtin_admin(None, now);
        let old_version = admin.version_meta().value();

        let _ = admin.delete(Some(UserId::generate().as_uuid()), now);

        assert_eq!(admin.version_meta().value(), old_version);
    }

    /// 无操作人删除（系统自动清理场景）
    #[test]
    fn test_delete_without_operator() {
        let now = base_now();
        let mut user = build_normal_test_user(None, now);

        user.delete(None, now).unwrap();

        assert!(user.is_deleted());
        assert_eq!(user.delete_meta().deleted_by(), None);
    }

    // ===================== 11. update_info 补充边界测试 =====================

    /// 已禁用用户禁止自主更新资料
    #[test]
    fn test_update_info_disabled_reject() {
        let now = base_now();
        let mut user = build_normal_test_user(None, now);
        user.status = Status::Disabled;

        let err = user
            .update_info(
                "new_name".into(),
                Email::new("a@b.com").unwrap(),
                Phone::new("13800138000").unwrap(),
                Some(UserId::generate().as_uuid()),
                now,
            )
            .unwrap_err();

        assert!(matches!(err, DomainError::UserSuspended { .. }));
    }

    /// 更新资料失败时版本号不应递增
    #[test]
    fn test_update_info_fail_no_version_bump() {
        let now = base_now();
        let mut admin = build_builtin_admin(None, now);
        let old_version = admin.version_meta().value();

        let _ = admin.update_info(
            "x".into(),
            Email::new("a@b.com").unwrap(),
            Phone::new("13800138000").unwrap(),
            Some(UserId::generate().as_uuid()),
            now,
        );

        assert_eq!(admin.version_meta().value(), old_version);
    }
}
