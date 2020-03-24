# Authentication
## Root
JJS has builtin root user, with username "$root" (Note that $ prefix can't be used in non-special user accounts).
You can not log into this user using primary api.
### Development root user
When JJS is running in development mode, you can authenticate as root, using "dev_root" string as access token
This option is automatically disabled in non-development environment, and shouldn't be turned on forcefully.
### Local Root login service in Linux
This server is started automatically by apiserver and is bound to `/tmp/jjs-auth-sock`.
You should connect to this socket from a process, uid of which exactly matches uid of apiserver.
Local auth server will write string of form `===S===`, where S is root access token.
## Normal users
See [`createUser`](https://mikailbag.github.io/jjs/api/mutation.doc.html).