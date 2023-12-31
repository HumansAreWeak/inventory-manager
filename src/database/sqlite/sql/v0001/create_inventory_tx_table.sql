-- This file is part of invman.
--
-- invman - Manage your inventory easily, declaratively, without the headache.
-- Copyright (C) 2023  Maik Steiger <m.steiger@csurielektronics.com>
--
-- invman is free software: you can redistribute it and/or modify
-- it under the terms of the GNU General Public License as published by
-- the Free Software Foundation, either version 3 of the License, or
-- (at your option) any later version.
--
-- invman is distributed in the hope that it will be useful,
-- but WITHOUT ANY WARRANTY; without even the implied warranty of
-- MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
-- GNU General Public License for more details.
--
-- You should have received a copy of the GNU General Public License
-- along with invman. If not, see <https://www.gnu.org/licenses/>.
CREATE TABLE invman_inventory_tx(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dispatcher INTEGER NOT NULL,
    schema_id INTEGER NOT NULL,
    inventory_id INTEGER NOT NULL,
    action_no INTEGER NOT NULL,
    from_val TEXT DEFAULT NULL,
    to_val TEXT DEFAULT NULL,
    created_at TEXT DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    FOREIGN KEY(dispatcher) REFERENCES invman_users(id),
    FOREIGN KEY(schema_id) REFERENCES invman_inventory_schema_tx(id)
);
