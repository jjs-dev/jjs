# Working with Frontend

## Overview 
Frontend exposes HTTP REST-like API

## Typings
API is strongly typed. You should look at `frontend-api/src/lib.rs` for more details.
Trait `Frontend` is not used in code base in any way, it only exists for API user convenience. 
Each function, declared in this trait, corresponds to API endpoint. 

## Authentication
All API methods from non-`auth` section, and `auth/drop` (TODO: not implemented) require request to be authenticated.
Authentication can be performed in various ways, e.g. see `auth/anonymous` and `auth/simple`.
These methods return you `AuthToken` object, containing token.
This token should be provided in all subsequent requests, as value of header `X-JJS-Auth`.

## Logout
(TODO: not implemented)
Just call `auth/drop` with desired token as `X-JJS-Auth` header value, and token will be revoked.