# Working with Frontend

## Overview 
Frontend exposes GraphQL API.
Assuming frontend is running on `127.0.0.1:1779`, visit `http://localhost:1779/graphiql` for playground.

## Authentication
All API methods from non-`auth` category, and authDrop (TODO: not implemented) require request to be authenticated.
Authentication can be performed in various ways, e.g. see `authAnonymous` and `authSimple`.
These methods return you `AuthToken` object, containing token.
This token should be provided in all subsequent requests, as value of header `X-JJS-Auth`.

## Logout
(TODO: not implemented)
Just call `authDrop` with desired token as `X-JJS-Auth` header value, and token will be revoked.