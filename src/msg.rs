use std::sync::Arc;

use hyper::{http::response::Builder as ResponseBuilder, Body, Method, Request};
use madome_sdk::auth::{self, Auth, Role, MADOME_ACCESS_TOKEN, MADOME_REFRESH_TOKEN};
use serde::de::DeserializeOwned;
use util::{
    http::{
        url::{is_path_variable, PathVariable},
        Cookie, SetHeaders,
    },
    r#async::AsyncTryFrom,
    ReadChunks,
};

use crate::{
    config::Config,
    usecase::{create_user, get_user},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Not found")]
    NotFound,
    #[error("Json deserialize")]
    JsonDeserializePayload(serde_json::Error),
}

/// Msg의 Payload는 같은 이름의 usecase의 Payload와는 관계가 없음
///
/// Msg의 Payload는 실행되어야하는 usecase 순서에 따라 정해짐 (제일 처음 실행하는 usecase의 Payload)
///
/// 실행되는 순서는 Resolver 참조
pub enum Msg {
    GetUser(get_user::Payload),
    CreateUser(create_user::Payload),
}

impl Msg {
    pub async fn http(
        request: Request<Body>,
        mut response: ResponseBuilder,
        config: Arc<Config>,
    ) -> crate::Result<(Self, ResponseBuilder)> {
        use Role::*;

        let method = request.method().clone();
        let path = request.uri().path();
        let cookie = Cookie::from(request.headers());

        let access_token = cookie.get(MADOME_ACCESS_TOKEN).unwrap_or_default();
        let refresh_token = cookie.get(MADOME_REFRESH_TOKEN).unwrap_or_default();

        let auth = Auth::new(
            config.madome_auth_url(),
            auth::has_public(request.headers()),
        );

        let (r, msg) = match (method, path) {
            (Method::POST, "/users") => {
                let (_, maybe_token_pair) = auth
                    .check_and_refresh(access_token, refresh_token, Developer(None))
                    .await?;

                let p = Wrap::async_try_from(request).await?.inner();

                (maybe_token_pair, Msg::CreateUser(p))
            }
            (Method::GET, path) if path == "/users/@me" || matcher(path, "/users/:user_id") => {
                match path {
                    "/users/@me" => {
                        let (r, maybe_token_pair) = auth
                            .check_and_refresh(access_token, refresh_token, Normal(None))
                            .await?;

                        let p = get_user::Payload {
                            id_or_email: r.user_id,
                        };

                        (maybe_token_pair, Msg::GetUser(p))
                    }
                    _ => {
                        let p: get_user::Payload =
                            PathVariable::from((path, "/users/:user_id")).into();

                        // TODO: 일단은 id만 됨. 이메일로 접근하는 방법은 아마 internal_auth를 통해서만 제공됨
                        let (_, maybe_token_pair) = auth
                            .check_and_refresh(
                                access_token,
                                refresh_token,
                                Normal(Some(&p.id_or_email)),
                            )
                            .await?;

                        (maybe_token_pair, Msg::GetUser(p))
                    }
                }
            }
            _ => return Err(Error::NotFound.into()),
        };

        if let Some(set_cookie) = r {
            // response에 쿠키 설정하고 response 넘겨줌
            response = response.headers(set_cookie.iter());
        }

        Ok((msg, response))
    }
}

fn matcher(req_path: &str, pattern: &str) -> bool {
    let mut origin = req_path.split('/');
    let pats = pattern.split('/');

    for pat in pats {
        if let Some(origin) = origin.next() {
            if !is_path_variable(pat) && pat != origin {
                return false;
            }
        } else {
            return false;
        }
    }

    origin.next().is_none()
}

pub struct Wrap<P>(pub P);

impl<P> Wrap<P> {
    pub fn inner(self) -> P {
        self.0
    }
}

#[async_trait::async_trait]
impl<P> AsyncTryFrom<Request<Body>> for Wrap<P>
where
    P: DeserializeOwned,
{
    type Error = crate::Error;

    async fn async_try_from(mut request: Request<Body>) -> Result<Self, Self::Error> {
        let chunks = request.body_mut().read_chunks().await?;

        let payload =
            serde_json::from_slice::<P>(&chunks).map_err(Error::JsonDeserializePayload)?;

        Ok(Wrap(payload))
    }
}
