--TEST--
pcntl CLI process control compatibility slice
--EXTENSIONS--
pcntl
--FILE--
<?php
$pid = pcntl_fork();
if ($pid === 0) {
    exit(23);
}

$status = -1;
$waited = pcntl_waitpid($pid, $status);
$waited_child = $waited === $pid;
$child_exited = pcntl_wifexited($status);
$child_exit_status = pcntl_wexitstatus($status);
$child_signaled = pcntl_wifsignaled($status);
$child_term_signal = pcntl_wtermsig($status);

echo extension_loaded('pcntl') ? "loaded\n" : "missing\n";
echo function_exists('pcntl_fork') ? "fork function\n" : "no fork\n";
echo function_exists('pcntl_exec') ? "exec function\n" : "no exec\n";
var_dump(defined('SIGUSR1'));
var_dump(defined('WNOHANG'));

var_dump(pcntl_async_signals());
var_dump(pcntl_async_signals(true));
var_dump(pcntl_async_signals());
var_dump(pcntl_signal(SIGUSR1, SIG_IGN));
var_dump(pcntl_signal_get_handler(SIGUSR1));
var_dump(pcntl_signal(SIGUSR1, 'strlen'));
var_dump(pcntl_signal_get_handler(SIGUSR1));
var_dump(pcntl_signal_dispatch());
var_dump(pcntl_alarm(0));

echo $waited_child ? "waited child\n" : "wait failed\n";
var_dump($child_exited);
var_dump($child_exit_status);
var_dump($child_signaled);
var_dump($child_term_signal);
var_dump(pcntl_errno());
?>
--EXPECTF--
loaded
fork function
exec function
bool(true)
bool(true)
bool(false)
bool(false)
bool(true)
bool(true)
int(1)
bool(true)
string(6) "strlen"
bool(true)
int(%d)
waited child
bool(true)
int(23)
bool(false)
bool(false)
int(0)
