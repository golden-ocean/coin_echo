use std::collections::HashMap;

use sqlx::PgPool;
use uuid::Uuid;

use crate::{error::AppError, queries::error::QueryError};

#[derive(Debug, Clone)]
pub struct OrganizationListItem {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub code: String,
    pub contact: String,
    pub phone: String,
    pub email: String,
    pub sort: i32,
    pub remark: Option<String>,
    pub status: String,
}

/// 树形节点：在扁平的 OrganizationListItem 基础上，额外挂一个 children 集合
#[derive(Debug, Clone)]
pub struct OrganizationTreeNode {
    pub item: OrganizationListItem,
    pub children: Vec<OrganizationTreeNode>,
}

/// 全量查询组织表，不分页、不带任何过滤条件——
/// 数据量小（几十到几百条），一次性取出后在内存里分组，比递归 CTE 更简单也更易维护
pub async fn handle_organization_list(
    pool: &PgPool,
) -> Result<Vec<OrganizationListItem>, AppError> {
    let items = sqlx::query_as!(
        OrganizationListItem,
        r#"
            SELECT id, parent_id, name, code, contact, phone, email, sort, remark, status
            FROM org_organization
            WHERE deleted_at IS NULL
            ORDER BY sort ASC, created_at ASC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(QueryError::from)?;

    Ok(items)
}

/// 在扁平列表基础上组装成树形结构。
///
/// 算法：先按 parent_id 分组建立索引，再从顶级节点（parent_id 为 NULL）开始递归拼装。
/// 这里的“递归”是内存中对象图的递归组装，和数据库层面的递归查询是两回事——
/// 前者只是普通函数调用栈，没有额外的数据库往返，代价可以忽略。
pub fn build_organization_tree(items: Vec<OrganizationListItem>) -> Vec<OrganizationTreeNode> {
    let mut children_map: HashMap<Option<Uuid>, Vec<OrganizationListItem>> = HashMap::new();
    for item in items {
        children_map.entry(item.parent_id).or_default().push(item);
    }

    fn build(
        parent_id: Option<Uuid>,
        map: &mut HashMap<Option<Uuid>, Vec<OrganizationListItem>>,
    ) -> Vec<OrganizationTreeNode> {
        let Some(siblings) = map.remove(&parent_id) else {
            return Vec::new();
        };

        siblings
            .into_iter()
            .map(|item| {
                let children = build(Some(item.id), map);
                OrganizationTreeNode { item, children }
            })
            .collect()
    }

    build(None, &mut children_map)
}

/// 组合上面两步：一次查询 + 一次内存建树，供 API 层直接调用
pub async fn handle_organization_tree(
    pool: &PgPool,
) -> Result<Vec<OrganizationTreeNode>, AppError> {
    let items = handle_organization_list(pool).await?;
    Ok(build_organization_tree(items))
}
