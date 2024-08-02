# Version 0.1.2 - Released 2024-08-TDB
working on it...

## cronframe 0.1.2
Changes
- `scheduler` method renamed to `start_scheduler` in CronFrame type
- using macros now doesn't require to install additional dependencies to your project except for the linkme crate
- the web server ip address is now configurable in the cronframe.toml
- the graceful period is now configurable in the cronframe.toml
- individual job scheduling can now be suspended and reprised from the job page
- the list of jobs in the homepage now categorises jobs as: active, timed-out, suspended

Additions
- added `new_job` method to CronFrame to create jobs without using macros
- added `keep_alive` method to CronFrame keep the main thread running
- added `stop_scheduler` method to CronFrame
- added `run` method in CronFrame which calls `start_schduler` and `keep_alive`
- added dark theme switcher 
- added scheduler status to the web pages
- added 5 seconds autoreload toggle to the web pages
- added modal to start and stop the scheduler from the web pages using [tinglejs](https://tingle.robinparisi.com/)

Fixes
- removed warnings that orignated from macros
- **BUGFIX**: the proper local time is now displayed in upcoming schedule
- **BUGFIX**: the absence of a section in the cronframe.toml no longer makes the configuration read fail
- **BUGFIX**: the policy for log files is now complied with after a restart

## cronframe_macro 0.1.2
- macros now use dependecies exported by the cronframe crate with che cronframe:: qualifier

# Version 0.1.1 - Released 2024-07-23
- Crate dependency fixes