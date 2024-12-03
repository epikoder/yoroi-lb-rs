use std::{error::Error, fmt::Debug};

use postgrest::Postgrest;
use tracing::info;

#[derive(Debug, Default)]
pub struct Service;

impl Service {
    pub(crate) async fn _register(
        &self,
        email: String,
        password: String,
    ) -> Result<(), Box<dyn Error>> {
        let client = Postgrest::new("http://localhost:9000");

        let user_already_exist = client
            .from("users")
            .select("id")
            .eq("email", email.clone())
            .single()
            .execute()
            .await?
            .status()
            .as_u16()
            .eq(&200);
        if user_already_exist {
            return Err("user already exist".into());
        }
        let password = match crypto::hash::make(password) {
            Ok(s) => s,
            Err(err) => return Err(err.to_string().into()),
        };

        let resp = client
            .from("users")
            .insert(format!(
                r#"{{"email": "{}", "password": "{}"}}"#,
                email, password
            ))
            .execute()
            .await?;
        match resp.status().as_u16() {
            200 => (),
            _ => return Err("err".into()),
        }

        let r = resp.text().await?;
        info!(r);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{thread::sleep, time::Duration};

    use super::*;

    #[tokio::test]
    async fn test_register() {
        tracing_subscriber::fmt().init();
        let r = Service::default()
            ._register("acme@gmail.com".to_string(), "password".to_string())
            .await;
        println!("{:?}", r);
        sleep(Duration::from_secs(1));
    }
}
