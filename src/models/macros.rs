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
macro_rules! reply {
    ($ctx: expr, $msg: expr) => {
        {
            if msg.id == 69420 {
                // Slash command
            } else {
                msg.reply($ctx, $msg).await;
            }
        }
    }
}