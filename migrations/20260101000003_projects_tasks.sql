CREATE TABLE projects(id UUID PRIMARY KEY,name TEXT NOT NULL,description TEXT,owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,created_at TIMESTAMPTZ NOT NULL DEFAULT now(),updated_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE INDEX projects_owner_idx ON projects(owner_id);
CREATE TABLE tasks(id UUID PRIMARY KEY,project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,title TEXT NOT NULL,description TEXT,status TEXT NOT NULL DEFAULT 'TODO' CHECK(status IN ('TODO','IN_PROGRESS','BLOCKED','DONE')),priority SMALLINT NOT NULL DEFAULT 3 CHECK(priority BETWEEN 1 AND 5),due_date DATE,created_at TIMESTAMPTZ NOT NULL DEFAULT now(),updated_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE INDEX tasks_project_status_idx ON tasks(project_id,status);
CREATE INDEX tasks_due_date_idx ON tasks(due_date);
