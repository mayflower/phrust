<?php
// runtime-semantics: category=properties expect=pass php_ref_required=1
// Declared-property storage surface: visibility, typed, readonly,
// dynamic, unset/re-set ordering, clone, and var_dump labels must be
// identical to the reference under slot-backed storage.
// (#[AllowDynamicProperties] avoids the dynamic-property deprecation
// notice, which the engine does not emit yet — separate known gap.)
#[AllowDynamicProperties]
class Dto {
    public $pub = 1;
    protected $prot = "p";
    private $priv = [1, 2];
    public int $typed = 7;
    public readonly string $ro;

    public function __construct() {
        $this->ro = "locked";
    }

    public function poke() {
        $this->prot = "poked";
        $this->priv[] = 3;
        return count($this->priv);
    }
}

$d = new Dto();
var_dump($d);

echo $d->poke(), "|", $d->pub, "|", $d->typed, "\n";

$d->dyn = "added";
echo $d->dyn, "\n";

unset($d->pub);
$d->pub = 99;
var_dump($d);

$copy = clone $d;
$copy->pub = 1000;
$copy->dyn = "copy";
echo $d->pub, "|", $copy->pub, "|", $d->dyn, "|", $copy->dyn, "\n";

echo isset($d->missing) ? "set" : "missing", "\n";
unset($d->dyn);
echo isset($d->dyn) ? "set" : "unset", "\n";
var_dump($d);
