# heft

Shows you where your disk space went and what you can delete.

## why

If youve ever run out of disk space and spent an hour figuring out what to delete, this is for you. node_modules from projects you forgot about, cargo target directories that somehow grew to 5gb, python venvs everywhere. Its always the same stuff but you have to hunt for it every time.

heft finds it all in one scan.

## status

Work in progress. The project artifact scanner works. Cache detection and docker support coming next.

## usage

```
heft scan --roots ~/code
```

Finds node_modules, cargo targets, python venvs, pycache, gradle builds, xcode deriveddata, go vendor dirs. Shows you the size of each and which ones are safe to delete.

## building

```
cargo build
cargo test
```

## whats next

- cache detector (npm, pip, cargo, homebrew caches)
- docker detector (images, volumes, build cache)
- cleanup command with dry run
- snapshot history so you can see what grew back

## license

MIT
