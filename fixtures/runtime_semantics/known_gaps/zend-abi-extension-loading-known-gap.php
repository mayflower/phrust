<?php
// runtime-semantics: category=known_gaps expect=known_gap known_gap=E_PHP_RUNTIME_UNSUPPORTED_ZEND_ABI
// PHP reference: extension metadata is exposed through the Zend extension registry.
echo extension_loaded("json") ? "json-loaded\n" : "json-absent\n";
