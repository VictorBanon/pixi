use std::time::Instant;

use rusqlite::{Connection, Result as SqlResult};

pub struct Timer {
    inicio: Instant,
    activo: bool,
}

pub struct TodoItem {
    pub id: i32,
    pub text: String,
    pub checked: bool,
    tiempo_acumulado: i32,
    temporizador: Option<Timer>,
}

impl TodoItem {
    pub fn tiempo_total(&self) -> i32 {
        if let Some(ref timer) = self.temporizador {
            if timer.activo {
                return self.tiempo_acumulado + timer.inicio.elapsed().as_secs() as i32;
            }
        }
        self.tiempo_acumulado
    }

    pub fn temporizador_activo(&self) -> bool {
        self.temporizador.as_ref().map_or(false, |t| t.activo)
    }

    pub fn pausar_temporizador(&mut self, db: &Db) {
        if let Some(ref timer) = self.temporizador {
            self.tiempo_acumulado += timer.inicio.elapsed().as_secs() as i32;
            let _ = db.actualizar_tiempo(self.id, self.tiempo_acumulado);
        }
        self.temporizador = None;
    }

    pub fn iniciar_temporizador(&mut self) {
        self.temporizador = Some(Timer {
            inicio: Instant::now(),
            activo: true,
        });
    }

    pub fn resetear_temporizador(&mut self, db: &Db) {
        self.tiempo_acumulado = 0;
        self.temporizador = None;
        let _ = db.actualizar_tiempo(self.id, 0);
    }
}

pub struct Db {
    conn: Connection,
}

impl Db {
    /// Open or create the database and initialize tables
    pub fn new(path: &str) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init()?;
        Ok(db)
    }

    /// Initialize the table and insert sample data if empty
    fn init(&self) -> SqlResult<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS tareas (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                descripcion TEXT NOT NULL,
                completada INTEGER DEFAULT 0,
                tiempo_acumulado INTEGER DEFAULT 0
            )",
            [],
        )?;

        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM tareas", [], |row| row.get(0))?;

        if count == 0 {
            let ejemplos = [
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

            for tarea in ejemplos {
                self.agregar_tarea(tarea)?;
            }
        }

        Ok(())
    }

    pub fn cargar_tareas(&self) -> SqlResult<Vec<TodoItem>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, descripcion, completada, tiempo_acumulado FROM tareas")?;

        let tareas_iter = stmt.query_map([], |row| {
            Ok(TodoItem {
                id: row.get(0)?,
                text: row.get(1)?,
                checked: row.get::<_, i32>(2)? != 0,
                tiempo_acumulado: row.get::<_, i32>(3)?,
                temporizador: None,
            })
        })?;

        Ok(tareas_iter.filter_map(|t| t.ok()).collect())
    }

    pub fn agregar_tarea(&self, descripcion: &str) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO tareas (descripcion) VALUES (?1)",
            [descripcion],
        )?;
        Ok(())
    }

    pub fn eliminar_tarea(&self, id: i32) -> SqlResult<()> {
        self.conn
            .execute("DELETE FROM tareas WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn actualizar_tarea(&self, id: i32, completada: bool) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE tareas SET completada = ?1 WHERE id = ?2",
            [completada as i32, id],
        )?;
        Ok(())
    }

    pub fn actualizar_tiempo(&self, id: i32, tiempo: i32) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE tareas SET tiempo_acumulado = ?1 WHERE id = ?2",
            [tiempo, id],
        )?;
        Ok(())
    }

    pub fn actualizar_descripcion(&self, id: i32, descripcion: &str) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE tareas SET descripcion = ?1 WHERE id = ?2",
            [descripcion, &id.to_string()],
        )?;
        Ok(())
    }
}
