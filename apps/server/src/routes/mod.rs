//! 路由汇总

pub mod health;
pub mod openapi;

// pub fn build(state: Arc<AppState>) -> Router {
//     let router: Router<Arc<AppState>> = Router::new().merge(health::router());

//     // 中间件对状态类型泛型化，挂在 with_state 之前之后都可以，
//     // 这里选择在注入 state 之前挂，方便中间件里将来如果需要访问
//     // AppState 时不必再改动这里的调用顺序。
//     let router = platform_middleware::apply(router);

//     router
//         .with_state(state) // Router<Arc<AppState>> -> Router<()>
//         .merge(openapi::router()) // 不需要状态，必须在这一步之后合并
// }
