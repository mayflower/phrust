# WordPress Smoke Documentation

WordPress is a representative real-world workload for Phrust. These documents
track request, bootstrap, filesystem, database, and performance observations
for that workload.

WordPress-specific patches and Phrust WordPress workarounds are out of scope.
Issues found here must be reduced to generic PHP language, runtime, standard
library, SAPI, filesystem, database, or performance behavior.

## Smoke Status

- [Bootstrap status](bootstrap-status.md)
- [Language and VM core status](language-vm-core-status.md)
- [Real smoke](real-smoke.md)
- [Bring-up diagnostics](bringup/web-db-diagnostics.md)
- [Autoload and standard-library pack](bringup/pack-b-autoload-stdlib.md)

## Performance

- [Root profiling](root-profiling.md)
- [Root performance report](root-performance-report.md)
- [Root performance follow-up](root-performance-followup.md)
