<?php
// stdlib-diff: id=STDLIB_PROCESS_CAPABILITY_DISABLED area=process expect=skip
echo function_exists('proc_open') ? "proc-open-symbol\n" : "missing\n";
echo function_exists('shell_exec') ? "shell-symbol\n" : "missing\n";
echo shell_exec('echo disabled') === false ? "shell-disabled\n" : "bad\n";
echo exec('echo disabled') === false ? "exec-disabled\n" : "bad\n";
echo system('echo disabled') === false ? "system-disabled\n" : "bad\n";
echo passthru('echo disabled') === false ? "passthru-disabled\n" : "bad\n";
$pipes = [];
echo proc_open('echo disabled', [], $pipes) === false ? "proc-disabled\n" : "bad\n";
echo popen('echo disabled', 'r') === false ? "popen-disabled\n" : "bad\n";
