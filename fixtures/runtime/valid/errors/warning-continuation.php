<?php
// phase4-runtime: expect=known_gap known_gap=E_PHP_RUNTIME_WARNING_CHANNEL_COMPAT
// phase4: kind=valid expected_stdout="ok\n" diagnostic_id=E_PHP_RUNTIME_UNDEFINED_VARIABLE_WARNING
echo $missing;
echo "ok\n";
