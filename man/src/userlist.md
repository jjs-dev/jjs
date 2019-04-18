# Userlist
Userlist is a tool, which helps you managing users
## Adding users
1. Prepare file, containing users data.
2. Run `jjs-userlist add path/to/file`. For additional options (including authentication), run `jjs-userlist --help`

File format:
Each line is handled separately. It can be
- A comment. Such line must start from # character
- User entry. It must contain login and password, both base64-encoded, separated with whitespace
