<?php
trait AliasTrait {
    public function run() { return 'run'; }
}

class AliasTraitBox {
    use AliasTrait {
        run as protected;
        run as private hidden;
    }

    public function call() { return $this->run() . '|' . $this->hidden(); }
}

echo (new AliasTraitBox())->call(), "\n";
