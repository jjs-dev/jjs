# Userlist
Userlist is a tool, which helps you managing users.
## Adding users
1. Prepare file, containing users data.
2. Run `jjs-userlist add path/to/file`. For additional options (including authentication), run `jjs-userlist --help`.

File format:
Each line is handled separately. It can be
- Configuration update entry.
- User entry. It must contain login, password, and groups.
### User Entry Format
`add <USERNAME> <PASSWORD> [SETTINGS]`
### Header Entry Format
`cfg SETTINGS` 
### Supported Options and Flags
Setting are comma-separated.
To enable flag, pass it's name.
To set option, use `option_name=option_value`
Currently, following flags are supported:
 - `base64` - if enabled, `userlist` decodes all other entities (logins, passwords, groups) in file from base64.
 This option is recommended to use if login, password or other piece of information can contain non-ASCII chars.
 - `ignore-fail` - if enabled, `userlist` will continue creating users, ignoring possible errors.
Following options are supported:
 - `groups` (takes colon-separated list of groups) - adds groups to implicit list. When user is created, 
 it is added to all groups from this list.
 - `set-groups` - same as `groups`, but clears that implicit list instead of appending.