use serde::{Deserialize, Serialize};

/// 兼容 Spring Data Sort 的 JSON 结构
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Sort {
    pub empty: bool,
    pub sorted: bool,
    pub unsorted: bool,
}

impl Sort {
    /// 创建默认的 unsorted Sort（与 Spring Data 的默认行为一致）
    pub fn unsorted() -> Self {
        Self {
            empty: true,
            sorted: false,
            unsorted: true,
        }
    }
}

/// 兼容 Spring Data Pageable 的 JSON 结构
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Pageable {
    pub page_number: i64,
    pub page_size: i64,
    pub sort: Sort,
    pub offset: i64,
    pub paged: bool,
    pub unpaged: bool,
}

/// 兼容 Spring Data Page 的 JSON 结构
///
/// 将 RBatis 的 `Page<T>` 转换为此结构后，序列化输出的 JSON 与 Java Spring Data `Page<T>` 完全一致。
///
/// # 使用示例
///
/// ```rust,ignore
/// use genies_core::page::SpringPage;
/// use rbatis::plugin::page::{Page, PageRequest};
///
/// // 从 RBatis Page 转换
/// let rbatis_page: Page<MyEntity> = /* 查询结果 */;
/// let spring_page: SpringPage<MyVO> = SpringPage::from_rbatis_page(rbatis_page, |e| e.into());
/// ```
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SpringPage<T> {
    pub content: Vec<T>,
    pub pageable: Pageable,
    pub last: bool,
    pub total_pages: i64,
    pub total_elements: i64,
    pub first: bool,
    pub size: i64,
    pub number: i64,
    pub sort: Sort,
    pub number_of_elements: i64,
    pub empty: bool,
}

impl<T> SpringPage<T> {
    /// 从 RBatis Page 转换为 SpringPage，同时转换记录类型。
    ///
    /// # 参数
    /// - `page`: RBatis 分页查询结果
    /// - `convert`: 记录类型转换函数（如 Entity → VO）
    pub fn from_rbatis_page<E: Send + Sync>(
        page: rbatis::plugin::page::Page<E>,
        convert: impl Fn(E) -> T,
    ) -> Self {
        let page_no = page.page_no as i64;
        let page_size = page.page_size as i64;
        let total = page.total as i64;
        let total_pages = if page_size > 0 {
            (total + page_size - 1) / page_size
        } else {
            0
        };
        let number = page_no - 1; // Spring Data 页码从 0 开始，RBatis 从 1 开始
        let content: Vec<T> = page.records.into_iter().map(convert).collect();
        let number_of_elements = content.len() as i64;
        let sort = Sort::unsorted();

        SpringPage {
            content,
            pageable: Pageable {
                page_number: number,
                page_size,
                sort: sort.clone(),
                offset: number * page_size,
                paged: true,
                unpaged: false,
            },
            last: number >= total_pages - 1,
            total_pages,
            total_elements: total,
            first: number == 0,
            size: page_size,
            number,
            sort,
            number_of_elements,
            empty: number_of_elements == 0,
        }
    }
}

impl<T: Send + Sync> From<rbatis::plugin::page::Page<T>> for SpringPage<T> {
    fn from(p: rbatis::plugin::page::Page<T>) -> Self {
        Self::from_rbatis_page(p, |e| e)
    }
}
