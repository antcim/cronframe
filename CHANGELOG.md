# Version tdb - Released tdb
working on it...

## cronframe tdb
- using macros now doesn't require to install additional dependencies to you many project except for the linkme crate
- ip address is now configurable in the cronframe.toml
- BUGFIX: the proper local time is now displayed in upcoming schedule
- BUGFIX: the absence of either or both the webserver and logger sections in the `cronframe.toml` no longer makes the config read fail

## cronframe_macro tdb
- macros now use dependecy types/functions/macros exported by the cronframe crate with che cronframe:: qualifier

# Version 0.1.1 - Released 2024-07-23
- Crate dependency fixes