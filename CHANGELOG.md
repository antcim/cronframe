# Version tdb - Released tdb
working on it...

## cronframe tdb
- added dark theme switcher 
- added 5 seconds page autoreload toggle
- using macros now doesn't require to install additional dependencies to your project except for the linkme crate
- the web server ip address is now configurable in the cronframe.toml
- the graceful period is now configurable in the cronframe.toml
- added `run method` in CronFrame type to start the scheduler and keep the main thread running
- individual job scheduling can now be suspended and reprised from the job page
- the list of jobs in the homepage now separates suspended jobs from active ones as well as timed-out jobs
- BUGFIX: the proper local time is now displayed in upcoming schedule
- BUGFIX: the absence of a section in the cronframe.toml no longer makes the config read fail
- BUGFIX: the policy for log files is now complied with after a restart

## cronframe_macro tdb
- macros now use dependecy types/functions/macros exported by the cronframe crate with che cronframe:: qualifier

# Version 0.1.1 - Released 2024-07-23
- Crate dependency fixes