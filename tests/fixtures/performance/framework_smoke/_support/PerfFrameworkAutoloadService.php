<?php
class PerfFrameworkAutoloadService {
    private $prefix;

    public function __construct($prefix) {
        $this->prefix = $prefix;
    }

    public function handle($name) {
        return $this->prefix . ':' . strtolower($name);
    }
}
