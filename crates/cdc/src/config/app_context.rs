use crate::config::app_config::ApplicationConfig;

pub struct ServiceContext {
    pub config: ApplicationConfig,
    // pub cache_service: CacheService,
    // pub redis_save_service: CacheService,
    // pub sys_res_service: SysResService,
    // pub sys_user_service: SysUserService,
    // pub sys_role_service: SysRoleService,
    // pub sys_role_res_service: SysRoleResService,
    // pub sys_user_role_service: SysUserRoleService,
    // pub sys_dict_service: SysDictService,
}

impl Default for ServiceContext {
    fn default() -> Self {
        let config = ApplicationConfig::default();

        ServiceContext {
            // keycloak_keys: task::block_on(async {
            //     get_keycloak_keys(&config.keycloak_auth_server_url, &config.keycloak_realm).await
            // }),
            //
            // rbatis: task::block_on(async { crate::config::init_rbatis(&config).await }),
            // cache_service: CacheService::new(&config),
            // redis_save_service: CacheService::new_saved(&config),
            // sys_res_service: SysResService {},
            // sys_user_service: SysUserService {},
            // sys_role_service: SysRoleService {},
            // sys_role_res_service: SysRoleResService {},
            // sys_user_role_service: SysUserRoleService {},
            // sys_dict_service: SysDictService {},
            config,
        }
    }
}
