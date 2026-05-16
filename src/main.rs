<?php
header("Access-Control-Allow-Origin: *");
header("Access-Control-Allow-Headers: Content-Type");
header("Content-Type: application/json");

// 1. CREDENCIALES DE TU PGADMIN4 EN RENDER
$host = "dpg-d83rb8t7vvec73el1p0g-a.oregon-postgres.render.com"; 
$port = "5432";                       
$user = "admin_yogurt";      
$pass = "1jme7epC5J59K167DK0eN5OPwfBBe1xM"; // <-- Pon tu clave de Render aquí
$db = "yogurt_db"; 

// Aquí Render sí tiene el driver instalado nativamente
$connection_string = "host=$host port=$port dbname=$db user=$user password=$pass";
$conn = pg_connect($connection_string);

if (!$conn) {
    echo json_encode(["status" => "error", "message" => "Fallo de conexión"]);
    exit;
}

$action = $_GET['action'] ?? '';

// ACCIÓN: ENVIAR PRODUCTOS A LA TIENDA
if ($action === 'productos') {
    $result = pg_query($conn, "SELECT * FROM productos ORDER BY id ASC");
    $arr = [];
    while($row = pg_fetch_assoc($result)) { $arr[] = $row; }
    echo json_encode($arr);
    exit;
}

// ACCIÓN: LOGIN / REGISTRO
if ($action === 'login' && $_SERVER["REQUEST_METHOD"] == "POST") {
    $email = pg_escape_string($conn, $_POST['email']);
    $password = $_POST['password'];
    $result = pg_query_params($conn, "SELECT * FROM clientes WHERE email = $1", array($email));
    
    if (pg_num_rows($result) > 0) {
        $user_data = pg_fetch_assoc($result);
        if ($password === $user_data['password']) { 
            echo json_encode(["status" => "success", "user" => $user_data]);
        } else {
            echo json_encode(["status" => "error", "message" => "Contraseña incorrecta"]);
        }
    } else {
        $nombre = pg_escape_string($conn, $_POST['nombre'] ?? 'Cliente Nuevo');
        $insert = pg_query_params($conn, "INSERT INTO clientes (nombre, email, password) VALUES ($1, $2, $3) RETURNING *", array($nombre, $email, $password));
        echo json_encode(["status" => "success", "user" => pg_fetch_assoc($insert)]);
    }
    exit;
}

// ACCIÓN: GUARDAR PAGO
if ($action === 'pago' && $_SERVER["REQUEST_METHOD"] == "POST") {
    $nombre = pg_escape_string($conn, $_POST['nombre']);
    $direccion = pg_escape_string($conn, $_POST['direccion']);
    $pago = pg_escape_string($conn, $_POST['pago']);
    $referencia = pg_escape_string($conn, $_POST['referencia']);
    $productos = pg_escape_string($conn, $_POST['productos_json']);
    $total = floatval($_POST['total_num']);

    $sql = "INSERT INTO historial_crm (nombre, direccion, metodo_pago, referencia, productos_comprados, total_pagado) VALUES ('$nombre', '$direccion', '$pago', '$referencia', '$productos', $total)";
    if (pg_query($conn, $sql)) { echo json_encode(["status" => "success"]); }
    exit;
}
?>

