# cleanr-cli

Prebuilt npm distribution of the `cleanr` terminal application.

```bash
npm install --global cleanr-cli
cleanr
```

The package installs a small Node.js launcher plus the native binary matching
the current operating system and CPU architecture. Native binary packages use
the `@cleanr-cli/<os>-<cpu>` naming pattern.

## Acknowledgements

Cleanr includes code adapted from
[Byron/dua-cli](https://github.com/Byron/dua-cli), an MIT-licensed disk usage
analyzer by Sebastian Thiel and contributors.
