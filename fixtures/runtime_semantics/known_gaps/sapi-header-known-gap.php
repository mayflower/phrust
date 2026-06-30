<?php
// runtime-semantics: category=known_gaps expect=known_gap known_gap=E_PHP_RUNTIME_UNSUPPORTED_SAPI
// PHP reference: CLI still exposes SAPI header functions with CLI-specific behavior.
header("X-Phrust-Fixture: yes");
echo headers_sent() ? "sent\n" : "pending\n";
