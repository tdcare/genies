/**
 * 动态获取 API 基础路径。
 *
 * 通过解析 window.location.pathname 中 /xxx/ui/ 的模式，
 * 提取 /xxx 作为部署前缀（即 nginx 的 servlet_path）。
 *
 * - 通过 nginx 访问 (http://localhost/auth-admin/ui/) → 返回 '/auth-admin'
 * - 直接访问后端 (http://localhost:9099/ui/)        → 返回 ''
 */
export function getApiBaseUrl(): string {
  const path = window.location.pathname
  // 动态获取部署前缀：匹配 /xxx/ui/ 格式，提取 /xxx 作为 API 前缀
  const match = path.match(/^(\/[^/]+)\/ui/)
  return match ? match[1] : ''
}
