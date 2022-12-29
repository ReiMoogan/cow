#[macro_export]
macro_rules! db {
    ($ctx: expr) => {
        {
            let ctx_global = $ctx.data.read().await;
            let out = ctx_global.get::<Database>().expect("Couldn't find database").clone();

            out
        }
    }
}

#[macro_export]
macro_rules! cowdb {
    ($ctx: expr) => {
        {
            db!($ctx.serenity_context())
        }
    }
}