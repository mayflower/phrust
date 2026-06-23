# ADR-0064: Phase 6 Composer Source Mode

## Status

Accepted for Phase 6.

## Context

Composer compatibility is a practical integration target, but `composer.phar`,
network installs, plugins, and scripts require broader PHAR, process, and
network support than Phase 6 requires.

## Decision

Composer source mode with local fixtures is the required Phase 6 gate. PHAR is
optional and must be decided separately. Online Packagist, plugins, and scripts
are excluded from required gates.

## Consequences

Composer work focuses first on local autoload, platform checks, source-mode
bootstrap, and prioritized missing-function reports.
