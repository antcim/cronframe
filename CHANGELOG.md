# Version 0.1.3 - Released ---
Web server functionality update, method updates and bugfixes.

## cronframe 0.1.3
**Additions**
- CronFrame is now also a CLI tool for spinning a global instance of the framework.
- Added support for running cli jobs that can be added with the new cli tool.

## cronframe_macro 0.1.3
- Derivation of Clone trait in the cron_obj macro itself.
- Method `cf_drop` in cron objects turned into an associated function and renamed `cf_drop_fn`.

<!-- version separator -->

# Version 0.1.2 - Released 2024-08-05
Web server functionality update, method updates and bugfixes.

## cronframe 0.1.2
**Changes**
- `scheduler` method renamed to `start_scheduler` in CronFrame type
- using macros now doesn't require to install additional dependencies to your project except for the linkme crate
- the web server ip address is now configurable in the cronframe.toml
- the graceful period is now configurable in the cronframe.toml
- individual job scheduling can now be suspended and reprised from the job page
- the list of jobs in the homepage now categorises jobs as: active, timed-out, suspended

**Additions**
- added `new_job` method to CronFrame to create jobs without using macros
- added `keep_alive` method to CronFrame keep the main thread running
- added `stop_scheduler` method to CronFrame
- added `run` method in CronFrame which calls `start_schduler` and `keep_alive`
- added dark theme switcher 
- added scheduler status to the web pages
- added 5 seconds autoreload toggle to the web pages
- added modal to start and stop the scheduler from the web pages using [tinglejs](https://tingle.robinparisi.com/)
- added new examples with a weather_alert job to show possible real usecase 

**Fixes**
- removed warnings that orignated from macros
- **BUGFIX**: the proper local time is now displayed in upcoming schedule
- **BUGFIX**: the absence of a section in the cronframe.toml no longer makes the configuration read fail
- **BUGFIX**: the policy for log files is now complied with after a restart

## cronframe_macro 0.1.2
- macros now use dependecies exported by the cronframe crate with che cronframe:: qualifier

<!-- version separator -->

# Version 0.1.1 - Released 2024-07-23
- Crate dependency fixes