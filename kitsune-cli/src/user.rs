use crate::{Result, Error};
use clap::{Subcommand};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use kitsune_db::{schema::{users, accounts}, model::{
    account::{NewAccount, ActorType},
    user::{User, NewUser},
}};
use speedy_uuid::Uuid;
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection};
use iso8601_timestamp::Timestamp;
use rsa::{
    pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding},
    RsaPrivateKey,
};

const TOKEN_LENGTH: usize = 32;

#[inline]
#[must_use]
pub fn generate_secret() -> String {
    use hex_simd::{AsOut, AsciiCase};

    let token_data: [u8; TOKEN_LENGTH] = rand::random();
    let mut buf = [0_u8; TOKEN_LENGTH * 2];
    (*hex_simd::encode_as_str(&token_data, buf.as_mut_slice().as_out(), AsciiCase::Lower))
        .to_string()
}

#[derive(Subcommand)]
pub enum UserSubcommand {
    /// Add a user
    Add {
        /// Username for the user you want to add
        username: String,
        /// Email address for the user you want to add
        email: String,
        /// Password for the user y u want to add
        password: String,
    },

    /// Mark a user as confirmed
    Confirm {
        /// Username for the user you want to mark as confirmed
        username: String,
    },

    /// List all users
    List,
}

async fn add_user(db_conn: &mut AsyncPgConnection, username: String, email: String, password: String) -> Result<()> {
    let hashed_password_fut = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(rand::thread_rng());
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
    });
    let private_key_fut =
        tokio::task::spawn_blocking(|| RsaPrivateKey::new(&mut rand::thread_rng(), 4096));

    let (hashed_password, private_key) = tokio::join!(hashed_password_fut, private_key_fut);
    let hashed_password = hashed_password??;

    let private_key = private_key??;
    let public_key_str = private_key.as_ref().to_public_key_pem(LineEnding::LF)?;
    let private_key_str = private_key.to_pkcs8_pem(LineEnding::LF)?;

    let account_id = Uuid::now_v7();
    let domain = "".to_string();
    let url = "".to_string();
    let public_key_id = "".to_string();
    // let domain = url_service.domain().to_string();
    // let url = url_service.user_url(account_id);
    // let public_key_id = url_service.public_key_id(account_id);

    db_conn
        .transaction(|tx| {
            async move {
                diesel::insert_into(accounts::table)
                    .values(NewAccount {
                        id: account_id,
                        display_name: None,
                        username: username.as_str(),
                        locked: false,
                        note: None,
                        local: true,
                        domain: domain.as_str(),
                        actor_type: ActorType::Person,
                        url: url.as_str(),
                        featured_collection_url: None,
                        followers_url: None,
                        following_url: None,
                        inbox_url: None,
                        outbox_url: None,
                        shared_inbox_url: None,
                        public_key_id: public_key_id.as_str(),
                        public_key: public_key_str.as_str(),
                        created_at: None,
                    })
                    .execute(tx).await?;

                let confirmation_token = generate_secret();
                diesel::insert_into(users::table)
                    .values(NewUser {
                        id: Uuid::now_v7(),
                        account_id,
                        username: username.as_str(),
                        oidc_id: None,
                        email: email.as_str(),
                        password: Some(hashed_password.as_str()),
                        domain: domain.as_str(),
                        private_key: private_key_str.as_str(),
                        confirmation_token: confirmation_token.as_str(),
                    })
                    .execute(tx).await?;

                Ok::<_, Error>(())
            }
            .scope_boxed()
        })
        .await?;

    Ok(())
}

async fn confirm_user(db_conn: &mut AsyncPgConnection, username_str: &str) -> Result<()> {
    use kitsune_db::schema::{
        users::dsl::username,
    };

    diesel::update(users::table.filter(username.eq(username_str)))
        .set(users::confirmed_at.eq(Timestamp::now_utc()))
        .execute(db_conn)
        .await?;

    Ok(())
}

async fn list_users(db_conn: &mut AsyncPgConnection) -> Result<()> {
    let users = users::table
        .load::<User>(db_conn)
        .await?;

    println!("Users:");
    for user in users {
        println!("- {:?} (added at: {})", user.username, user.created_at);
    }

    Ok(())
}

pub async fn handle(cmd: UserSubcommand, db_conn: &mut AsyncPgConnection) -> Result<()> {
    match cmd {
        UserSubcommand::Add { username, email, password } => add_user(db_conn, username, email, password).await?,
        UserSubcommand::Confirm { username } => confirm_user(db_conn, &username).await?,
        UserSubcommand::List => list_users(db_conn).await?,
    }

    Ok(())
}
