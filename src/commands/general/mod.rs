mod info;
mod rank;
mod ban;

use serenity::framework::standard::macros::group;

use info::*;
use rank::*;
use ban::*;

#[group]
#[commands(info, rank, disablexp, levels, bangenshinplayers, banleagueplayers, banvalorantplayers, banoverwatchplayers)]
#[description = "General commands for miscellaneous tasks."]
#[summary = "Basic commands"]
struct General;
