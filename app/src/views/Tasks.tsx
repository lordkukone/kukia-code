export default function Tasks() {
  return (
    <div className="page">
      <div className="page-head">
        <h1>Tareas</h1>
        <span className="chip warn">sin tareas en segundo plano</span>
      </div>
      <div className="card">
        <h2>Administrador de tareas</h2>
        <p className="muted">
          En esta fase no hay ejecuciones en segundo plano: cada acción (chat, benchmark, comando)
          se ejecuta y termina de forma síncrona. El registro de aprobaciones y la auditoría sí
          quedan guardados en SQLite.
        </p>
        <p className="muted">
          Las tareas largas (investigación autónoma, descargas, actualizaciones) llegarán en fases
          posteriores con control de progreso y cancelación.
        </p>
      </div>
    </div>
  );
}
