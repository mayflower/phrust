<?php
class PerfFrameworkDto {
    public $id;
    public $name;
    public $active;

    public function label() {
        return $this->id . ':' . strtoupper($this->name) . ':' . ($this->active ? 'yes' : 'no');
    }
}

$rows = array(
    array('id' => 1, 'name' => 'alpha', 'active' => true),
    array('id' => 2, 'name' => 'beta', 'active' => false),
    array('id' => 3, 'name' => 'gamma', 'active' => true),
);

foreach ($rows as $row) {
    $dto = new PerfFrameworkDto();
    $dto->id = $row['id'];
    $dto->name = $row['name'];
    $dto->active = $row['active'];
    echo $dto->label(), "\n";
}
