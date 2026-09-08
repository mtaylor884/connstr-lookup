# connstr

A small command-line tool for pulling one field out of a connection string.

## The problem

Connection strings for things like SQL Server or ODBC data sources are
usually a flat list of `key=value` pairs separated by semicolons:

```
Server=.;Database=app;User Id=sa;Password=hunter2;Encrypt=true
```

The obvious way to grab one field from a string like this is
`s.split(';')` followed by `split('=')`. That works right up until a
value needs to contain a `;` or a `=`, at which point it has to be
quoted:

```
Server=.;Database=app;Password='needs; a semicolon';Encrypt=true
```

A naive split breaks on the semicolon inside the quotes and either
truncates the password or misaligns every field after it. `connstr`
parses the actual grammar - quoting with `'` or `"`, doubled quotes as
an escape for a literal quote character, and whitespace trimming -
instead of guessing with string splits.

## Usage

Build it with `cargo build --release`; the binary ends up at
`target/release/connstr`.

Get one field:

```
$ echo "Server=.;Database=app;User Id=sa;Password=hunter2" | connstr get - Database
app
```

List every key found, in the order they appear:

```
$ echo "Server=.;Database=app;User Id=sa" | connstr keys -
Server
Database
User Id
```

Key lookup is case-insensitive, matching how these connection strings
are actually interpreted (`connstr get - server` and
`connstr get - Server` return the same thing).

`get` also knows about the common synonym groups these strings use in
practice, so it doesn't matter which spelling a particular driver
picked:

```
$ echo "Data Source=.;Uid=sa;Pwd=hunter2" | connstr get - Server
.
$ echo "Data Source=.;Uid=sa;Pwd=hunter2" | connstr get - "User Id"
sa
```

The groups covered are Server/Address/Addr/Network Address/Data
Source, Database/Initial Catalog, User Id/Uid/User/Username/User
Name, and Password/Pwd. `keys` still lists whichever spelling
actually appears in the string, unchanged.

Check a string for problems without caring about any particular key:

```
$ echo "Server=.;Database=app;=oops;Timeout" | connstr validate -
-: line 1, column 23: empty key before '='
-: line 1, column 29: entry 'Timeout' has no '=' before the next ';'
error: 2 errors found
```

Unlike `get` and `keys`, which stop at the first problem they hit,
`validate` keeps scanning after an error so it can report everything
wrong with the string in one pass.

`FILE` can be a real file path instead of `-`, which is the more
common case: connection strings usually live in a config file rather
than getting typed on a command line.

## Scripting with --format json

Every command accepts `--format json` (in place anywhere on the
command line) for output a script can parse instead of the plain-text
form:

```
$ echo "Server=.;Database=app" | connstr get --format json - Database
{"key":"Database","value":"app"}

$ echo "Server=.;Database=app" | connstr keys --format json -
{"keys":["Server","Database"]}

$ echo "Server=.;=oops" | connstr validate --format json -
{"errors":[{"message":"empty key before '='","line":1,"column":11}]}
```

On success `get` and `keys` print their JSON object to stdout;
`validate` prints `{"status":"ok"}`. On failure - a missing key, a
malformed string - the JSON error object goes to stderr instead of the
plain-text message, and the process still exits non-zero.

## Errors point at the exact character

Splitting on punctuation by hand tends to produce error messages like
"invalid connection string" with no indication of where the problem
is. `connstr` tracks line and column as it scans, so a malformed
string gets a precise answer:

```
$ echo "Server=.;Database=app;Password='unterminated;Timeout=30" | connstr get - Password
error: -: line 1, column 32: value starting here is opened with ' but never closed
```

## Known limitations

This covers the common shape of these strings, not the entire .NET
`DbConnectionStringBuilder` grammar. In particular it does not yet
handle doubled `=` as an escape inside an unquoted key. See the issue
tracker for what's planned next.

## License

MIT, see [LICENSE](LICENSE).
