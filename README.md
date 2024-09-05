# rtodo
rtodo is terminal task manager based on the [todo.txt format](https://github.com/todotxt/todo.txt)

# Installation
#Todo

## Usage
rtodo is a simple binary and doesn't add anything else to the user's system. But at start, it checks for a configuration file in user's configuration directory and loads it. There is an example Configuration file in [config.toml](config.toml)
rtodo will look at 3 thing at start up to search for a file to Open.
1. A file path as a default argument `rtodo dir/todo.txt`
2. `file_path` value in the configuration file in default location if present.
3. Look for a file called todo.txt in the current directory

You can also pass a configuration file as an argument using the `-c` flag.  which can also specify it's own file path

## A List of Shorcuts
`d` or `space` - Toggle Done for the Task
`x` - Delete Task
`j` or 🡣 - Move to next task
`k` or 🡩 - Move to prev task
`n` - Start writing a new task
`e` - Edit the current task
`/` - start the search input
`l` - load a search
`a` - save a search to be reused later
`~` - Help
`:` - Goto mode similar to vim or helix
`Ctrl+d` - Clear out the current input in search or while editing a task

## Searching 
You can start search by typing '/'.
A search starting with `-` can be used to ignore a particular string. E.g. Searching with '-@context' will filter out any task that contain `@context`.
A `,` seperated list for some advanced searching. So using 'done @context' will not match a task like `A done task with @context`. But searching with 'done,@context' will match the same task

## Recurring Task
`rec` key can be used for making a recurrent task. Both a `rec` and `due` need to be present for it to work. Completing a recurrent Task will create a new Task with a due date based on the current task. The value for recurring can be rec:+10d:
`+` indicated that the calculation of the next due date needs to be strict.
- Strict here means that the next due date will be calculated based on the lat Due date
- Not Strict will lead to due date being calculated from the current date, date of completion of the current Task

Lastly the alphabet at the end indicated the Unit of Time so:
- 'd' is for Days 
- 'w' for Weeks
- 'm' for Months
- 'y' for Years
- No letter at the end will considered as Days

You can create Birthday reminder like so 'Alan's Birthday due:2024-08-15 rec:+1y'
