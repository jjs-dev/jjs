# Clients

JJS itself (i.e. `frontend`) doesn't provide any user interface. 
Instead, it has [Thrift](https://thrift.apache.org)-based RPC API (see `frontend_api` for more details).
Clients are independent apps built on top of this API, which _have_ UI.

## Client list

[webclient](https://github.com/mikailbag/jjs/tree/master/webclient) - official, very simple