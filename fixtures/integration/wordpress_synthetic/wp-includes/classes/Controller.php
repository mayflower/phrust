<?php
namespace Synthetic;

class Controller {
    public function render(): void {
        echo "controller\n";
        if (getenv('PHRUST_MYSQL_TEST_DSN') !== false) {
            echo "db-capability\n";
        }
    }
}
