--TEST--
PHPT runner ENV smoke
--ENV--
PHRUST_PHPT_RUNNER_ENV=runner-env
--FILE--
<?php
echo getenv("PHRUST_PHPT_RUNNER_ENV"), "\n";
--EXPECT--
runner-env
