use hyper::{Body, Response, StatusCode};

use crate::{
    model::Presenter,
    payload,
    usecase::{
        create_like, create_notifications, create_or_update_fcm_token, create_user, delete_like,
        get_fcm_tokens, get_likes, get_likes_from_book_tags, get_notifications, get_user,
    },
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Msg: {0}")]
    Msg(#[from] MsgError),
    #[error("Command: {0}")]
    Command(#[from] CommandError),
    #[error("UseCase: {0}")]
    UseCase(#[from] UseCaseError),
    #[error("Repository: {0}")]
    Repository(#[from] RepositoryError),

    #[error("Payload: {0}")]
    Payload(#[from] payload::Error),

    #[error("AuthSdk: {0}")]
    AuthSdk(#[from] madome_sdk::auth::Error),

    // TODO: 나중에 위치 재선정
    #[error("ReadChunksFromBody: {0}")]
    ReadChunksFromBody(#[from] hyper::Error),
}

type MsgError = crate::msg::Error;

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("SeaOrm: {0}")]
    SeaOrm(#[from] sea_orm::DbErr),
}

impl From<sea_orm::DbErr> for crate::Error {
    fn from(error: sea_orm::DbErr) -> Self {
        Error::Repository(error.into())
    }
}

impl From<sea_orm::TransactionError<sea_orm::DbErr>> for crate::Error {
    fn from(err: sea_orm::TransactionError<sea_orm::DbErr>) -> Self {
        match err {
            sea_orm::TransactionError::Connection(err) => Self::Repository(err.into()),
            sea_orm::TransactionError::Transaction(err) => Self::Repository(err.into()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CommandError {}

#[derive(Debug, thiserror::Error)]
pub enum UseCaseError {
    #[error("GetUser: {0}")]
    GetUser(#[from] get_user::Error),
    #[error("CreateUser: {0}")]
    CreateUser(#[from] create_user::Error),

    #[error("GetLikes: {0}")]
    GetLikes(#[from] get_likes::Error),
    #[error("CreateLike: {0}")]
    CreateLike(#[from] create_like::Error),
    #[error("DeleteLike: {0}")]
    DeleteLike(#[from] delete_like::Error),
    #[error("GetLikesFromBookTags: {0}")]
    GetLikesFromBookTags(#[from] get_likes_from_book_tags::Error),

    #[error("CreateNotifications: {0}")]
    CreateNotifications(#[from] create_notifications::Error),
    #[error("GetNotifications: {0}")]
    GetNotifications(#[from] get_notifications::Error),

    #[error("CreateOrUpdateFcmToken: {0}")]
    CreateOrUpdateFcmToken(#[from] create_or_update_fcm_token::Error),
    #[error("GetFcmTokens: {0}")]
    GetFcmTokens(#[from] get_fcm_tokens::Error),
}

impl Presenter for Error {
    fn to_http(self, response: hyper::http::response::Builder) -> Response<Body> {
        use crate::msg::Error::*;
        use create_like::Error::*;
        use create_user::Error::*;
        use delete_like::Error::*;
        use get_user::Error::*;
        use Error::*;
        use UseCaseError::*;

        match self {
            Msg(JsonDeserializePayload(err)) => response
                .status(StatusCode::BAD_REQUEST)
                .body(err.to_string().into()),

            Msg(NotFound) => response
                .status(StatusCode::NOT_FOUND)
                .body("Not found".into()),

            Payload(err) => response
                .status(StatusCode::BAD_REQUEST)
                .body(err.to_string().into()),

            UseCase(CreateUser(
                err @ InvalidName(_) | err @ InvalidEmail(_) | err @ InvalidRole(_),
            )) => response
                .status(StatusCode::BAD_REQUEST)
                .body(err.to_string().into()),

            UseCase(CreateUser(AlreadyExistsUser)) => response
                .status(StatusCode::CONFLICT)
                .body("Already exist user".into()),

            UseCase(GetUser(NotFoundUser)) => response
                .status(StatusCode::NOT_FOUND)
                .body("Not found user".into()),

            UseCase(CreateLike(err @ AlreadyExistsLike)) => response
                .status(StatusCode::CONFLICT)
                .body(err.to_string().into()),

            UseCase(DeleteLike(err @ NotFoundLike)) => response
                .status(StatusCode::NOT_FOUND)
                .body(err.to_string().into()),

            AuthSdk(err) => err.to_http(response),

            err => response
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(err.to_string().into()),
        }
        .unwrap()
    }
}
