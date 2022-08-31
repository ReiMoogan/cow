





mod pavilion;
mod pav_models;








use serenity::framework::standard::macros::group;






use pavilion::*;






#[group]
#[prefixes("ucm", "ucmerced")]
#[description = "Get information about UC Merced's pavilion."]
#[summary = "UC Merced info"]
#[commands(pavilion)]

struct UCM;
