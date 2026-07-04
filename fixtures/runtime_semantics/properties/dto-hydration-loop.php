<?php
// runtime-semantics: category=properties expect=pass php_ref_required=1
// DTO hydration loop: repeated constructor writes and method reads over
// declared properties, mirroring model-hydration workloads.
class Model {
    public $id = 0;
    public $name = "";
    public $score = 0.0;

    public function __construct($id, $name, $score) {
        $this->id = $id;
        $this->name = $name;
        $this->score = $score;
    }

    public function describe() {
        return $this->id . ":" . $this->name . ":" . $this->score;
    }
}

$rows = [];
for ($i = 1; $i <= 20; $i++) {
    $rows[] = new Model($i, "row$i", $i * 1.5);
}

$total = 0.0;
$parts = [];
foreach ($rows as $model) {
    $total += $model->score;
    if ($model->id % 7 === 0) {
        $parts[] = $model->describe();
    }
}
echo count($rows), "|", $total, "\n";
echo implode(",", $parts), "\n";

$magic = new class {
    private $bag = [];
    public function __get($name) {
        return $this->bag[$name] ?? "absent:$name";
    }
    public function __set($name, $value) {
        $this->bag[$name] = strtoupper($value);
    }
};
$magic->title = "quiet";
echo $magic->title, "|", $magic->nothing, "\n";
