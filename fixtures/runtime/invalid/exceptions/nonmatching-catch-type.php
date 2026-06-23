<?php
class DomainException {}

try {
    throw new Exception("boom");
} catch (DomainException $e) {
    echo "domain\n";
}
