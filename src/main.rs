use teloxide::prelude2::*;
use teloxide::utils::command::BotCommand;
use anyhow;
use std::env;
use teloxide::types::ChatId::ChannelUsername;
use teloxide::types::ForwardedFrom;
use crate::mongo::MongoDatabase;

mod mongo;

#[derive(BotCommand, Clone)]
#[command(rename = "lowercase", description = "Тільки для адмінів:")]
enum AdminCommand {
    #[command(
        rename = "fake",
        description = "бот повідомить якщо користувач перешле інформацію з санкційного пабліку."
    )]
    AddChatToRestriction(String),
    #[command(
    rename = "unfake",
    description = "видалити з санкційного списку"
    )]
    RemoveFromRestriction(String)
}

#[derive(BotCommand, Clone)]
#[command(rename = "lowercase", description = "Ми підтримуємо ці команди:")]
enum GeneralCommand {
    #[command(
    rename = "help",
    description = "показує цей текст"
    )]
    Help,
    #[command(
    rename = "restricted_list",
    description = "список санкціонних груп"
    )]
    RestrictedList
}

#[tokio::main]
async fn main() {
    teloxide::enable_logging!();
    log::info!("Starting dices_bot...");

    let bot = Bot::from_env().auto_send();
    run(bot).await;
}

async fn run(bot: AutoSend<Bot>) {
    let mongodb = create_db().await.unwrap();
    let bot_info = bot.get_me().await.unwrap();
    let bot_id = bot_info.user.id;
    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<GeneralCommand>()
                .endpoint(command_handler)
        )
        .branch(dptree::entry()
            .filter_command::<AdminCommand>()
            .endpoint(admin_command_handler))
        .branch(
            dptree::filter(|msg: Message| {
                msg.forward_from().is_some()
            }).endpoint(forward_handle),
        ).branch(
        dptree::filter(move |msg: Message| {
            msg.new_chat_members().map(|users| users.into_iter().any(|user|user.id == bot_id)).unwrap_or(false)
        }).endpoint(join_chat),
    );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![mongodb])
        .default_handler(|_|async{})
        .error_handler(LoggingErrorHandler::with_custom_text(
            "An error has occurred in the dispatcher",
        ))
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;
}

async fn forward_handle(message: Message, bot: AutoSend<Bot>, db: mongo::MongoDatabase) -> anyhow::Result<()> {
    if let Some(forwarded) = message.forward_from() {
        match forwarded {
            ForwardedFrom::Chat(chat) => {
                if db.is_chat_restricted(message.chat_id(), chat.id).await? {
                    bot.send_message(message.chat_id(), "ОБЕРЕЖНО: група розповсюджує фейки").reply_to_message_id(message.id).await?;
                };
            }
            _ => {
                // bot.send_message(message.chat_id(), "Не довіряйте інформації від користувачів. Вона не є перевіреною.").reply_to_message_id(message.id).await?;
            }
        }
    }
    Ok(())
}
async fn command_handler(message: Message,bot: AutoSend<Bot>,
                               command: GeneralCommand,
                               db: mongo::MongoDatabase) -> anyhow::Result<()> {
    match command {
        GeneralCommand::Help =>{ bot.send_message(message.chat_id(), format!(
            "{}\n{}",
            GeneralCommand::descriptions(),
            AdminCommand::descriptions()
        )).reply_to_message_id(message.id).await?; Ok(())},
        GeneralCommand::RestrictedList => show_restricted_list(db, bot, message.chat_id()).await
    }
}

async fn admin_command_handler(message: Message,
                         bot: AutoSend<Bot>,
                         command: AdminCommand,
                        db: mongo::MongoDatabase) -> anyhow::Result<()> {
    let admins = bot.get_chat_administrators(message.chat_id()).await?;
    if ! message.from()
        .map(|user| admins.into_iter().any(|member| member.user.id == user.id) )
        .unwrap_or(false) {
        bot.send_message(message.chat_id(), "Упс, а ти точно головнокомандувач? (адмін)").reply_to_message_id(message.id).await?;
        Ok(())
    }
    else {
        let chat_id = message.chat_id();
        match command {
            AdminCommand::AddChatToRestriction(user) => add_to_restrictions(db, bot, chat_id,user).await,
            AdminCommand::RemoveFromRestriction(user) => remove_from_restrictions(db, bot, chat_id, user).await
        }?;
        Ok(())
    }
}

async fn create_db() -> Result<MongoDatabase, anyhow::Error> {
    if let Ok(con) = env::var("MONGO_CONNECTION_STRING") {
        if let Ok(database) = env::var("MONGO_DB_NAME") {
            return MongoDatabase::from_connection_string(&con, &database).await;
        }
    }
    panic!("TIKTOK_BOT_MONGO_CON_STRING & TIKTOK_BOT_DATABASE_NAME env variables don't exist");
}

async fn join_chat(message: Message, db: MongoDatabase, bot: AutoSend<Bot>) -> anyhow::Result<()> {
    static PREDEFINED_LIST: &'static [(&'static i64, &'static str)] = &[
        (&-1001263953596i64,&"grey_zone"),
        (&-1001085671411i64,&"rlz_the_kraken"),
        (&-1001144404150i64,&"warjournaltg"),
        (&-1001394050290i64,&"bbbreaking"),
        (&-1001074354585i64,&"milinfolive"),
        (&-1001459051914i64,&"Ugolok_Sitha"),
        (&-1001515682915i64,&"informator_life"),
        (&-1001108403053i64,&"chesnokmedia"),
        (&-1001510269826i64,&"ghost_of_novorossia"),
        (&-1001441564230i64,&"notes_veterans"),
        (&-1001162306948i64,&"diplomatia"),
        (&-1001658927353i64,&"bulbe_de_trones"),
        (&-1001324060080i64,&"olegtsarov"),
        (&-1001463486789i64,&"akimapache"),
        (&-1001437814721i64,&"zola_of_renovation"),
        (&-1001495313358i64,&"Hard_Blog_Line"),
        (&-1001659445882i64,&"ice_inii"),
        (&-1001144180066i64,&"swodki"),
        (&-1001111348665i64,&"infantmilitario"),
        (&-1001036362176i64,&"rt_russian"),
        (&-1001486063104i64,&"gazetaru"),
        (&-1001099860397i64,&"rbc_news"),
        (&-1001075565753i64,&"vedomosti"),
        (&-1001050820672i64,&"tass_agency"),
        (&-1001126629353i64,&"kremlinprachka"),
        (&-1001355540894i64,&"RVvoenkor"),
        (&-1001099737840i64,&"rusvesnasu"),
        (&-1001135021433i64,&"wargonzo"),
        (&-1001293450423i64,&"oldminerkomi"),
        (&-1001164620271i64,&"julia_chicherina"),
        (&-1001254516102i64,&"nezhurka"),
        (&-1001331641097i64,&"donbassr"),
        (&-1001355540894i64,&"RVvoenkor"),
        (&-1001389031111i64,&"radlekukh"),
        (&-1001087900551i64,&"egorgalenko"),
        (&-1001123791123i64,&"yudenich"),
        (&-1001147203857i64,&"Marinaslovo"),
        (&-1001120807475i64,&"vladlentatarsky"),
        (&-1001136444638i64,&"SonOfMonarchy"),
        (&-1001239133328i64,&"MaximYusin"),
        (&-1001205641526i64,&"istorijaoruzija"),
        (&-1001321128351i64,&"Govorit_Topaz"),
        (&-1001101806611i64,&"boris_rozhin"),
        (&-1001078100205i64,&"go338"),
        (&-1001457644020i64,&"omonmoscow"),
        (&-1001319370046i64,&"wingsofwar"),
        (&-1001362388271i64,&"chvkmedia"),
        (&-1001185457530i64,&"hackberegini"),
        (&-1001279747448i64,&"mig41"),
        (&-1001794711668i64,&"pezdicide"),
        (&-1001012103617i64,&"SergeyKolyasnikov"),
        (&-1001199117533i64,&"MedvedevVesti"),
        (&-1001246399946i64,&"SIL0VIKI"),
        (&-1001382288937i64,&"balkanossiper"),
        (&-1001577023152i64,&"pl_syrenka"),
        (&-1001376818144i64,&"brussinf"),
        (&-1001263152428i64,&"lady_north"),
        (&-1001339203072i64,&"sex_drugs_kahlo"),
        (&-1001297608614i64,&"usaperiodical"),
        (&-1001171552896i64,&"russ_orientalist"),
        (&-1001120807475i64,&"vladlentatarsky"),
        (&-1001210987817i64,&"neoficialniybezsonov"),
        (&-1001326223284i64,&"rybar")
    ];

    for (id, user) in PREDEFINED_LIST {
        db.add_to_restrictions(message.chat_id(), **id, user).await?;
    }
    bot.send_message(message.chat_id(), "Завантаження предефайнутих пабліків завершена").await?;
    Ok(())
}

async fn add_to_restrictions(db: MongoDatabase, bot: AutoSend<Bot>, chat_id: i64, user: String) -> anyhow::Result<()> {
    let username = if user.starts_with("@") {user.clone()} else {"@".to_owned() + &user};
    if let Ok(chat) = bot.get_chat(ChannelUsername(username)).await {
        db.add_to_restrictions(chat_id, chat.id, &user).await?;
        bot.send_message(chat_id, format!("{} воєнний корабль, іди нахуй", user)).await?;
    } else {
        bot.send_message(chat_id, format!("Щось не те з {}, перевір правильність даних", user)).await?;
    }
    Ok(())
}
async fn remove_from_restrictions(db: MongoDatabase, bot: AutoSend<Bot>, chat_id: i64, user: String) -> anyhow::Result<()> {
    db.remove_from_restrictions(chat_id, &user).await?;
    bot.send_message(chat_id, "Видалили з бази, або його там і не було").await?;
    Ok(())
}
async fn show_restricted_list(db: MongoDatabase, bot: AutoSend<Bot>, chat_id: i64) -> anyhow::Result<()> {
    let restrictions = db.restriction_list(chat_id).await?;
    if restrictions.is_empty() {
        bot.send_message(chat_id, "Ого, нікого не бачу. Що ж це ми всім довіряємо?").await?;
    } else {
        let restriction_string = restrictions.into_iter().fold(String::new(), |result, i| {
            result + &format!("@{}\n", i)
        });
        bot.send_message(chat_id, format!("Зараз в бані:\n\n{}", restriction_string)).await?;
    }
    Ok(())
}