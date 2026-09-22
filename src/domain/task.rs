//! Modelo y operaciones puras para la checklist.
//!
//! El dominio no conoce nada de la interfaz ni de la persistencia.  En
//! particular, `TaskId` no depende de la posición de una tarea: eliminar o
//! reordenar tareas no cambia sus identificadores.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Contador global usado como respaldo y para resolver dos creaciones en el
/// mismo instante. El valor se combina con el reloj para que los IDs nuevos
/// no vuelvan normalmente a empezar en uno al reiniciar la aplicación.
static LAST_TASK_ID: AtomicU64 = AtomicU64::new(0);

/// Identificador estable de una tarea.
///
/// Es deliberadamente opaco: el consumidor debe conservarlo para editar,
/// marcar, reordenar o eliminar una tarea, pero no debe derivar identidad de
/// su posición en la lista.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TaskId(u64);

impl TaskId {
    /// Crea un identificador a partir de un valor persistido.
    ///
    /// Esto resulta útil al cargar una checklist. Para crear IDs de nuevas
    /// tareas use `TaskId::new` indirectamente mediante `TaskList::add`.
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Alias legible para consumidores que prefieren el nombre `raw`.
    pub const fn raw(self) -> u64 {
        self.0
    }

    fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos().min(u64::MAX as u128) as u64)
            .unwrap_or(0);

        // CAS mantiene el valor estrictamente creciente incluso si el reloj
        // del sistema retrocede o si se crean varias tareas en paralelo.
        let mut previous = LAST_TASK_ID.load(Ordering::Relaxed);
        loop {
            let candidate = now.max(previous.saturating_add(1));
            match LAST_TASK_ID.compare_exchange_weak(
                previous,
                candidate,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Self(candidate),
                Err(current) => previous = current,
            }
        }
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Una tarea de la checklist.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Task {
    /// Identidad estable, independiente del orden actual.
    pub id: TaskId,
    /// Texto mostrado al usuario.
    pub text: String,
    /// Si la tarea fue marcada como completada.
    pub completed: bool,
    /// Posición consecutiva dentro de la lista, comenzando en cero.
    pub position: usize,
    /// Instante de creación como segundos desde Unix epoch.
    pub created_at: u64,
}

impl Task {
    fn new(text: impl Into<String>, position: usize) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());

        Self {
            id: TaskId::new(),
            text: text.into(),
            completed: false,
            position,
            created_at,
        }
    }
}

/// Colección ordenada de tareas y operaciones del checklist.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TaskList {
    tasks: Vec<Task>,
}

#[allow(dead_code)] // API de dominio usada por pruebas y extensiones futuras.
impl TaskList {
    /// Crea una checklist vacía.
    pub const fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    /// Crea una checklist a partir de tareas ya cargadas.
    ///
    /// Las tareas se ordenan por `position` y sus posiciones se normalizan.
    /// Este constructor es intencionalmente pequeño para que la capa de
    /// persistencia pueda reconstruir el dominio sin conocer sus invariantes.
    pub fn from_tasks(mut tasks: Vec<Task>) -> Self {
        tasks.sort_by_key(|task| task.position);
        let mut list = Self { tasks };
        list.normalize_positions();
        list
    }

    /// Devuelve la cantidad de tareas.
    pub const fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Indica si no hay tareas.
    pub const fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Vista inmutable de las tareas en el orden de presentación.
    pub fn as_slice(&self) -> &[Task] {
        &self.tasks
    }

    /// Alias de `as_slice` útil para capas de presentación y persistencia.
    pub fn tasks(&self) -> &[Task] {
        self.as_slice()
    }

    /// Consume la colección y devuelve sus tareas.
    pub fn into_tasks(self) -> Vec<Task> {
        self.tasks
    }

    /// Itera las tareas en el orden actual.
    pub fn iter(&self) -> impl Iterator<Item = &Task> {
        self.tasks.iter()
    }

    /// Busca una tarea por su identificador estable.
    pub fn get(&self, id: TaskId) -> Option<&Task> {
        self.tasks.iter().find(|task| task.id == id)
    }

    /// Busca una tarea mutable por su identificador estable.
    pub fn get_mut(&mut self, id: TaskId) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|task| task.id == id)
    }

    /// Agrega una tarea al comienzo y devuelve su identificador.
    ///
    /// Así la tarea recién creada queda visible inmediatamente sin que el
    /// usuario tenga que desplazarse hasta el final de la cola.
    pub fn add(&mut self, text: impl Into<String>) -> TaskId {
        let task = Task::new(text, 0);
        let id = task.id;
        self.tasks.insert(0, task);
        self.normalize_positions();
        id
    }

    /// Alias explícito para consumidores que prefieren el nombre verbal.
    pub fn add_task(&mut self, text: impl Into<String>) -> TaskId {
        self.add(text)
    }

    /// Reemplaza el texto de una tarea. Devuelve `false` si el ID no existe.
    pub fn edit(&mut self, id: TaskId, text: impl Into<String>) -> bool {
        let Some(task) = self.get_mut(id) else {
            return false;
        };
        task.text = text.into();
        true
    }

    /// Alias explícito para editar el texto.
    pub fn edit_task(&mut self, id: TaskId, text: impl Into<String>) -> bool {
        self.edit(id, text)
    }

    /// Invierte el estado completado y devuelve el nuevo valor.
    pub fn toggle(&mut self, id: TaskId) -> Option<bool> {
        let task = self.get_mut(id)?;
        task.completed = !task.completed;
        Some(task.completed)
    }

    /// Alias explícito para marcar/desmarcar una tarea.
    pub fn toggle_task(&mut self, id: TaskId) -> Option<bool> {
        self.toggle(id)
    }

    /// Elimina una tarea y devuelve si el ID existía.
    pub fn delete(&mut self, id: TaskId) -> bool {
        let Some(index) = self.tasks.iter().position(|task| task.id == id) else {
            return false;
        };
        self.tasks.remove(index);
        self.normalize_positions();
        true
    }

    /// Alias explícito para eliminar una tarea.
    pub fn delete_task(&mut self, id: TaskId) -> bool {
        self.delete(id)
    }

    /// Mueve una tarea a `new_position` y conserva el resto del orden.
    ///
    /// Si la posición supera el final, la tarea se coloca al final. Devuelve
    /// `false` si el ID no existe.
    pub fn reorder(&mut self, id: TaskId, new_position: usize) -> bool {
        let Some(old_position) = self.tasks.iter().position(|task| task.id == id) else {
            return false;
        };
        let task = self.tasks.remove(old_position);
        let destination = new_position.min(self.tasks.len());
        self.tasks.insert(destination, task);
        self.normalize_positions();
        true
    }

    /// Elimina las tareas completadas y devuelve cuántas quitó.
    pub fn clear_completed(&mut self) -> usize {
        let before = self.tasks.len();
        self.tasks.retain(|task| !task.completed);
        self.normalize_positions();
        before - self.tasks.len()
    }

    fn normalize_positions(&mut self) {
        for (position, task) in self.tasks.iter_mut().enumerate() {
            task.position = position;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(list: &TaskList) -> Vec<&str> {
        list.iter().map(|task| task.text.as_str()).collect()
    }

    #[test]
    fn add_assigns_stable_unique_ids_and_positions() {
        let mut list = TaskList::new();
        let first = list.add("primera");
        let second = list.add("segunda");

        assert_ne!(first, second);
        assert_eq!(texts(&list), vec!["segunda", "primera"]);
        assert_eq!(list.get(first).map(|task| task.position), Some(1));
        assert_eq!(list.get(second).map(|task| task.position), Some(0));
        assert!(list.get(first).is_some_and(|task| task.created_at > 0));
    }

    #[test]
    fn edit_and_toggle_change_only_the_requested_task() {
        let mut list = TaskList::new();
        let first = list.add("antes");
        let second = list.add("sin cambios");

        assert!(list.edit(first, "después"));
        assert_eq!(
            list.get(first).map(|task| task.text.as_str()),
            Some("después")
        );
        assert_eq!(list.toggle(first), Some(true));
        assert_eq!(list.toggle(first), Some(false));
        assert_eq!(list.get(second).map(|task| task.completed), Some(false));
        assert!(!list.edit(TaskId::from_raw(0), "inexistente"));
        assert_eq!(list.toggle(TaskId::from_raw(0)), None);
    }

    #[test]
    fn delete_normalizes_positions_without_changing_remaining_ids() {
        let mut list = TaskList::new();
        let first = list.add("uno");
        let second = list.add("dos");
        let third = list.add("tres");

        assert!(list.delete(second));
        assert_eq!(texts(&list), vec!["tres", "uno"]);
        assert_eq!(list.get(first).map(|task| task.position), Some(1));
        assert_eq!(list.get(third).map(|task| task.position), Some(0));
        assert!(!list.delete(second));
    }

    #[test]
    fn reorder_moves_task_and_clamps_position() {
        let mut list = TaskList::new();
        let first = list.add("uno");
        let _second = list.add("dos");
        let third = list.add("tres");

        assert!(list.reorder(first, 2));
        assert_eq!(texts(&list), vec!["tres", "dos", "uno"]);
        assert_eq!(list.get(first).map(|task| task.position), Some(2));
        assert!(list.reorder(first, usize::MAX));
        assert_eq!(texts(&list), vec!["tres", "dos", "uno"]);
        assert!(list.reorder(third, 0));
        assert_eq!(texts(&list), vec!["tres", "dos", "uno"]);
        assert!(!list.reorder(TaskId::from_raw(0), 0));
    }

    #[test]
    fn clear_completed_removes_only_completed_and_normalizes() {
        let mut list = TaskList::new();
        let first = list.add("pendiente");
        let second = list.add("terminada");
        let third = list.add("otra terminada");
        assert_eq!(list.toggle(second), Some(true));
        assert_eq!(list.toggle(third), Some(true));

        assert_eq!(list.clear_completed(), 2);
        assert_eq!(texts(&list), vec!["pendiente"]);
        assert_eq!(list.get(first).map(|task| task.position), Some(0));
        assert_eq!(list.clear_completed(), 0);
    }

    #[test]
    fn from_tasks_sorts_and_normalizes_positions() {
        let first = Task {
            id: TaskId::from_raw(1),
            text: "primera".into(),
            completed: false,
            position: 20,
            created_at: 1,
        };
        let second = Task {
            id: TaskId::from_raw(2),
            text: "segunda".into(),
            completed: true,
            position: 10,
            created_at: 2,
        };
        let list = TaskList::from_tasks(vec![first, second]);

        assert_eq!(texts(&list), vec!["segunda", "primera"]);
        assert_eq!(list.as_slice()[0].position, 0);
        assert_eq!(list.as_slice()[1].position, 1);
    }
}
