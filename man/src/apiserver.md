# Working with Apiserver

## Overview 
Apiserver exposes HTTP REST API.
Visit [JJS documantaion](https://jjs-dev.github.io/jjs/api) for api details.

## Authentication
All API methods from non-`auth` category, and authDrop (TODO: not implemented) require request to be authenticated.
Authentication can be performed in various ways, e.g. see `authAnonymous` and `authSimple`.
These methods return you `AuthToken` object, containing token.
This token should be provided in all subsequent requests:
`
// when logging in
let token: AuthToken = ...

// later, when doing request
req.set_header("Authorization", "Token ${token.data}")
`

## Logout
(TODO: not implemented)
Just call `authDrop` with desired token in `Authorization` header, and token will be revoked.