<?php
// runtime-fixture: expect=known_gap known_gap=E_PHP_RUNTIME_UNSUPPORTED_STDLIB category=UnsupportedFeature
echo image_type_to_mime_type(IMAGETYPE_PNG), "\n";
