# Inventory Manager

or `invman` as it is called as an executable, is a CLI tool for managing real-world inventories with high-level syntax. It is written to be as precise and as comprehensible as possible, i.e. it is possible to view each state that your inventory has been at a particular point in time. 

This tool is not yet production ready and is still in heavy development. 

# Features

* User Managment
  * Allow users to register and authenticate themselves when accessing the inventory
* Inventory Managment
  * Create, change or delete the schema you need for whatever it is you want to be storing (Internally items are referred to entities)
  * Create items that match the schema you have given to the program
  
# Planned features

- [ ] CRUD operations for inventory managment
- [ ] Permissions model with roles (with a guest and a skipper (root) role)

For any suggestions what could be implemented, open an issue or make a PR.

# Documentation

Since the program is really small for the time being, the documentation can be summarized in this Markdown.

### General

`invman <OPTIONS>`

Use this command to invoke the program. Additionally options may be parsed for other commands.

`Available Options:`
- `--auth / -a <username>:<password>`: Authenticate a request with the provided username and password and check, if the user is permitted to run a given command
- `--output / -o [json]`: Defines the output type at which the program will output its data into. For now only JSON is available.

### User Managment

`user register <username> <password>`

Registers a user with the username and the password.

### Inventory managment

`inventory schema alter --name <name> --column-type <type> <OPTIONS>`

Alters (Add / Edits) the inventory schema, the name is the column name of the table and column type its storage type. Additional options may be parsed, use the `-h` flag to list all available options.

`inventory schema remove --name <name>`

Removes a schema column from the inventory table. The schema column that is removed matches the given name.

`inventory schema list`

Lists the currently applied schema and outputs it

`inventory add <schema["name"]>=value...`

Adds an entity to inventory. Takes in a list of `schema["name"]=value` pairs. It sets the column to the given value in the database.

`inventory list <OPTIONS>`

Lists all the items in the inventory. You can give it options to further define the output that you want to list.
