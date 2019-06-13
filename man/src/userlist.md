# Userlist
Userlist is a tool, which helps you managing users
## Adding users
1. Prepare file, containing users data.
2. Run `jjs-userlist add path/to/file`. For additional options (including authentication), run `jjs-userlist --help`

File format:
Each line is handled separately. It can be
- Header entry. This entry must be first line in file
- A comment. Such line must start from # character
- User entry. It must contain login, password, and groups.
### User Entry Format
`USERNAME PASSWORD GROUP1:GROUP2...`
All items must be  separated by one color/whitespace as shown upper
Username, login and groups are separated with whitespace
### Header Entry Format
`! OPT1,OPT2,OPT3` (note, that options are _not_ separated by whitespace)

Currently, following options are supported:
 - `BASE64` - additionally decoded all other entities (logins, passwords, groups) in file from base64.
 This option is recommended to use if userlist can contain non-ASCII chars