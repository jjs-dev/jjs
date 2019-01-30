namespace * frontend_api

//auth
struct AuthToken {
    1: required binary buf,
}

enum AuthErrorType {
    IncorrectToken,
    AccessDenied,
}

exception AuthError {
    1: AuthErrorType type,
}

struct SimpleAuthParams {
    1: required string login,
    2: required string password,
}

enum SimpleAuthErrorType {
    UnknownLogin,
    IncorrectPassword,
    NotSuitable
}

exception SimpleAuthError {
    1: SimpleAuthErrorType type,
}

//submissions
struct SubmitDeclaration {
    1: required string toolchain,
    3: required binary code,
}

enum SubmitErrorType {
    UnknownToolchain,
    ContestIsOver,
    PermissionDenied,
    SizeLimitExceeded,
}
exception SubmitError {
    1: required SubmitErrorType type,
}

///MUST be non-negative
typedef i64 SubmissionId

service JjsService {
    //auth
    AuthToken anon(),
    AuthToken simple(1: SimpleAuthParams auth_params),
    void drop(1: AuthToken token) throws (1: AuthError auth_error),

    //submissions
    SubmissionId submit(1: SubmitDeclaration submit_params),

    //util
    string ping(1: string buf),
}