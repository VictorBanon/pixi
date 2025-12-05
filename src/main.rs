use eframe::egui;
use rusqlite::{Connection, Result as SqlResult};

fn main() -> Result<(), eframe::Error> {
    init_database().expect("Error al inicializar la base de datos");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 400.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "Lista de Tareas - SQLite",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

fn init_database() -> SqlResult<()> {
    let conn = Connection::open("tareas.db")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tareas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            descripcion TEXT NOT NULL,
            completada INTEGER DEFAULT 0
        )",
        [],
    )?;

    // Verificar si ya hay datos
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM tareas", [], |row| row.get(0))?;

    // Si no hay datos, insertar tareas de ejemplo
    if count == 0 {
        let tareas_ejemplo = vec![
            "Comprar leche y pan en el supermercado",
            "Llamar al dentista para cita",
            "Revisar correo electrónico importante",
            "Hacer ejercicio 30 minutos",
            "Leer capítulo del libro",
            "Preparar presentación para reunión",
            "Pagar facturas del mes",
            "Organizar escritorio de trabajo",
            "Estudiar Rust y egui",
            "Backup de archivos importantes",
        ];

        for tarea in tareas_ejemplo {
            conn.execute("INSERT INTO tareas (descripcion) VALUES (?1)", [tarea])?;
        }
    }

    Ok(())
}

fn cargar_tareas() -> SqlResult<Vec<TodoItem>> {
    let conn = Connection::open("tareas.db")?;
    let mut stmt = conn.prepare("SELECT id, descripcion, completada FROM tareas")?;

    let tareas = stmt.query_map([], |row| {
        Ok(TodoItem {
            id: row.get(0)?,
            text: row.get(1)?,
            checked: row.get::<_, i32>(2)? != 0,
        })
    })?;

    Ok(tareas.filter_map(|t| t.ok()).collect())
}

fn reventar_tareas() -> SqlResult<Vec<TodoItem>> {
    let conn = Connection::open("tareas.db")?;
    conn.execute("DELETE FROM tareas", [])?;
    Ok(vec![])
}

fn actualizar_tarea(id: i32, completada: bool) -> SqlResult<()> {
    let conn = Connection::open("tareas.db")?;
    conn.execute(
        "UPDATE tareas SET completada = ?1 WHERE id = ?2",
        [completada as i32, id],
    )?;
    Ok(())
}

fn agregar_tarea(descripcion: &str) -> SqlResult<()> {
    let conn = Connection::open("tareas.db")?;
    conn.execute(
        "INSERT INTO tareas (descripcion) VALUES (?1)",
        [descripcion],
    )?;
    Ok(())
}

struct TodoItem {
    id: i32,
    text: String,
    checked: bool,
}

struct MyApp {
    todos: Vec<TodoItem>,
    nueva_tarea: String,
}

impl Default for MyApp {
    fn default() -> Self {
        let todos = cargar_tareas().unwrap_or_else(|_| Vec::new());
        Self {
            todos,
            nueva_tarea: String::new(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("📋 Lista de Tareas - SQLite");

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            // Sección para agregar nueva tarea
            ui.horizontal(|ui| {
                ui.label("Nueva tarea:");
                let text_edit = ui.text_edit_singleline(&mut self.nueva_tarea);

                // Detectar Enter para agregar tarea
                if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if !self.nueva_tarea.trim().is_empty() {
                        if agregar_tarea(&self.nueva_tarea).is_ok() {
                            self.nueva_tarea.clear();
                            self.todos = cargar_tareas().unwrap_or_else(|_| Vec::new());
                        }
                    }
                }

                if ui.button("➕ Agregar").clicked() {
                    if !self.nueva_tarea.trim().is_empty() {
                        if agregar_tarea(&self.nueva_tarea).is_ok() {
                            self.nueva_tarea.clear();
                            self.todos = cargar_tareas().unwrap_or_else(|_| Vec::new());
                        }
                    }
                }
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            // Mostrar cada tarea de la base de datos como checkbox
            for todo in &mut self.todos {
                let checked_before = todo.checked;
                ui.checkbox(&mut todo.checked, &todo.text);

                // Si cambió el estado, actualizar en la base de datos
                if checked_before != todo.checked {
                    let _ = actualizar_tarea(todo.id, todo.checked);
                }
            }

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);

            // Estadísticas
            let total = self.todos.len();
            let completed = self.todos.iter().filter(|t| t.checked).count();
            let pending = total - completed;

            ui.label(format!("📊 Total: {}", total));
            ui.label(format!("✅ Completadas: {}", completed));
            ui.label(format!("⏳ Pendientes: {}", pending));

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("🔄 Recargar tareas").clicked() {
                    self.todos = cargar_tareas().unwrap_or_else(|_| Vec::new());
                }

                if ui.button("A tomar por...").clicked() {
                    self.todos = reventar_tareas().unwrap_or_else(|_| Vec::new());
                }
            });
        });
    }
}
