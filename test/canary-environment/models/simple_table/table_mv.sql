-- Copyright Materialize, Inc. and contributors. All rights reserved.
--
-- Use of this software is governed by the Business Source License
-- included in the LICENSE file at the root of this repository.
--
-- As of the Change Date specified in that file, in accordance with
-- the Business Source License, use of this software will be governed
-- by the Apache License, Version 2.0.

-- depends_on: {{ ref('table') }}
{{ config(materialized='materializedview', cluster='qa_canary_environment_compute', indexes=[{'default': True}]) }}

SELECT max(c) FROM {{ source('public_table','table' }}
