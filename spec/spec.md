# Merit Ledger MVP Spec

## 1. Product Summary

**Merit Ledger** is a private-first Buddhist-inspired practice ledger for recording wholesome actions, vows, precepts, repentance, merit dedication, and rejoicing in others’ merit.

The MVP freemium version is a local desktop app built with:

* **Python**
* **Pygame** frontend
* **FastAPI** local backend
* **DynamoDB-compatible local data model**
* **Tradition packs** for Zen, Chinese Mahayana, Nichiren, Pure Land, and Secular mode
* **Single-user local storage**
* Future-ready architecture for cloud sync, web UI, mobile UI, API keys, and paid hosting

The app should be beautiful, calming, non-shaming, non-performative, and spiritually useful without pretending to be a canonical arbiter of karma.

## 2. Product Principles

### 2.1 Private First

The MVP is local-only by default. The user’s ledger, vows, repentance entries, and dedications remain on their own machine.

No cloud account.
No telemetry.
No public feed.
No leaderboard.
No accidental confession.

### 2.2 Merit as Practice Accounting, Not Cosmic Accounting

The app should avoid claiming that it knows “real merit.” Points are user-defined practice counters.

Preferred language:

* Practice points
* Merit counter
* Dedication
* Rejoicing
* Return to practice
* Repair vow
* Renew intention

Avoid language like:

* Karma score
* Sin
* Impurity
* Spiritual failure
* You lost merit
* Bad Buddhist
* Confess your wrongdoing

### 2.3 Tradition-Aware, Not Sectarian

The MVP supports several day-one tradition profiles:

* Zen
* Chinese Mahayana
* Nichiren
* Pure Land
* Secular

Each tradition pack changes labels, defaults, suggested practices, iconography, and optional modules.

The app should let the user mix and match practices without forcing doctrinal purity.

### 2.4 No Secret Confession

Repentance is category-based. The app should explicitly discourage recording secrets, names, identifying details, illegal acts, sexual details, or private third-party information.

The repentance system is for:

* Acknowledgment
* Repair
* Intention
* Returning to practice

Not for storing a diary of shame.

### 2.5 Beauty Matters

The app should feel like a small digital shrine, not an admin dashboard.

Visual direction:

* Calm
* Spacious
* Warm
* Legible
* Low-friction
* Soft animations
* No casino gamification
* No dopamine leaderboard nonsense

## 3. MVP Scope

### 3.1 In Scope

The freemium MVP includes:

1. Local single-user profile
2. Tradition selection
3. Secular mode
4. Practice ledger
5. Positive vows
6. Negative vows
7. Vow pause/resume
8. Repentance entries
9. Merit dedication
10. Practice templates
11. User-customizable point values
12. Local “mudita demo” mode using sample/community-style entries
13. Basic charts
14. Export to JSON and Markdown
15. Import from JSON
16. Local FastAPI backend
17. Pygame UI
18. DynamoDB-shaped local storage adapter

### 3.2 Out of Scope for MVP

Not in the freemium MVP:

* Real multiuser cloud sync
* Real public feed
* ActivityPub federation
* Hosted accounts
* Mobile app
* Web UI
* Payment
* OAuth
* Email notifications
* API keys
* Teacher/admin dashboards
* Encrypted sync
* Real-time collaboration
* AI spiritual advice
* AI confession analysis
* Push notifications

### 3.3 Future Paid Version

The paid version may add:

* Hosted cloud storage
* Real DynamoDB backend
* Web UI
* Mobile UI
* API keys
* Cross-device sync
* Private groups
* Shared sanghas
* Mudita/rejoicing between users
* Group dedications
* Public/private profiles
* Optional ActivityPub federation
* API integrations
* Encrypted backups
* Premium tradition packs
* Teacher/mentor tools

## 4. Target User

Primary user:

* A Buddhist or Buddhist-adjacent practitioner who wants to track vows, precepts, good deeds, dedications, and repentance privately.

Secondary user:

* A secular person who wants a non-theistic virtue/habit ledger with gratitude, repair, restraint, and dedication-like reflections.

Developer assumption:

* MVP is built by one person.
* Local app should be shippable as a Python package or single-folder desktop app.
* Paid/cloud architecture should not require rewriting the domain model.

## 5. Core User Flows

## 5.1 First Launch

On first launch, user sees a beautiful onboarding flow:

1. Welcome screen
2. Choose mode:

   * Zen
   * Chinese Mahayana
   * Nichiren
   * Pure Land
   * Secular
   * Custom / Mixed
3. Choose point style:

   * Points enabled
   * Count-only
   * Reflection-only
4. Choose privacy reminder:

   * Local-only
   * Do not record secrets
5. Create local profile name
6. Enter main dashboard

Example welcome copy:

> Merit Ledger helps you record wholesome actions, vows, dedications, repentance, and rejoicing. It is a practice tool, not a judge.

## 5.2 Record a Positive Action

User clicks **Record Merit**.

Fields:

* Practice template
* Date/time
* Quantity
* Points
* Optional reflection
* Optional dedication
* Save

Example:

* Practice: Recited nembutsu
* Quantity: 108
* Points: 10
* Dedication: All sentient beings

Result:

* Ledger entry created
* Points added to daily/weekly total
* Optional dedication entry created

## 5.3 Record a Positive Vow Completion

User has a vow:

> Meditate for 10 minutes daily.

They click:

* Today’s Vows
* Mark Complete
* Optional reflection
* Save

Result:

* Ledger entry created
* Vow occurrence completed
* Points assigned
* Streak updated

## 5.4 Record a Negative Vow Breach

User has a vow:

> Refrain from harsh speech.

They click:

* Vow
* Record breach
* Select category
* Optional short note
* Choose repentance / repair action
* Save

The app should not ask:

> What exactly did you say? Who was involved?

Instead, it asks:

* Category: Harmful speech
* Reflection: optional, non-specific
* Repair intention: “I will pause before replying next time.”
* Repentance practice: selected from template

Result:

* Vow breach event created
* Repentance event created or scheduled
* Vow marked “repair in progress” until resolved
* No automatic shame score

## 5.5 Pause a Vow

User opens vow detail:

* Pause vow
* Reason:

  * Illness
  * Travel
  * Retreat
  * Overcommitted
  * Reframing vow
  * Other
* Resume date optional

Paused vows do not count as failed.

Button text:

> Pause with care

Not:

> Give up

## 5.6 Resume a Vow

User opens paused vow:

* Resume vow
* Optional new frequency
* Optional new point value
* Optional renewed intention

Result:

* Vow resumed
* Timeline shows pause/resume event

## 5.7 Repentance

User clicks **Return to Practice** or **Repentance**.

Options:

* Omnibus repentance
* Speech
* Harm
* Neglect
* Anger
* Greed
* Carelessness
* Broken vow
* Custom category

Flow:

1. Choose category
2. Choose repentance text/practice
3. Optional non-secret reflection
4. Choose repair intention
5. Dedicate merit
6. Save

Default UX copy:

> Keep this general. Do not record secrets, names, identifying details, illegal details, or private third-party information.

## 5.8 Dedicate Merit

User can dedicate merit after any positive entry or repentance entry.

Dedication targets:

* All sentient beings
* Family
* Ancestors
* The sick
* The dead
* Animals
* Beings suffering from war
* Beings suffering from hunger
* Someone specific
* Custom group

Fields:

* Target
* Dedication text
* Points dedicated
* Date/time

Important: dedication does not remove points by default. It records the dedication of the practice.

Optional advanced setting:

* Dedication reduces available balance
* Dedication does not reduce balance

Default: does not reduce balance.

## 5.9 Mudita Demo Mode

Because MVP is single-user, real social mudita is not available yet.

But the app can include a local “mudita garden” with sample anonymous wholesome actions. The user can rejoice in them.

Example sample entries:

* “Someone practiced patience during a difficult conversation.”
* “Someone fed birds and dedicated the merit to all beings.”
* “Someone recited the Lotus Sutra today.”
* “Someone sat zazen even though they felt restless.”
* “Someone called a lonely relative.”

User clicks:

* Rejoice
* Bow
* Sadhu
* Namu
* Anumodana

The label depends on tradition pack.

Result:

* Local ledger entry: “Rejoiced in another’s wholesome action.”
* Points assigned if enabled

This is a preview of future multiuser behavior without needing cloud accounts.

## 6. Tradition Packs

Tradition packs are JSON/YAML/Python data files that configure defaults.

Each pack includes:

* Display name
* Theme
* Icon set
* Suggested practices
* Suggested vows
* Repentance categories
* Dedication language
* Rejoicing language
* Point defaults
* Enabled/disabled modules
* Glossary terms

## 6.1 Zen Pack

Tone:

* Minimal
* Simple
* Spacious
* Practice-oriented

Suggested labels:

* Practice
* Vow
* Return to practice
* Dedication
* Rejoice
* Sit
* Bow

Suggested practices:

* Zazen
* Kinhin
* Bowing
* Chanting
* Cleaning practice space
* Mindful work
* Refraining from harsh speech
* Refraining from intoxication
* Eating mindfully
* Helping another person
* Rejoicing in another’s practice

Possible visual style:

* Ink wash
* Stone
* Paper
* Moss
* Empty circle / ensō-inspired iconography, but avoid copyrighted art

## 6.2 Chinese Mahayana Pack

Tone:

* Devotional
* Compassion-oriented
* Precept-aware
* Dedication-rich

Suggested labels:

* Merit
* Dedication of merit
* Repentance
* Precepts
* Bodhisattva practice
* Rejoicing

Suggested practices:

* Recite Heart Sutra
* Recite Amitabha Buddha’s name
* Recite Guanyin’s name
* Bowing repentance
* Vegetarian meal
* Release-life alternative: protect animals / donate to animal care
* Support monastics or temple
* Copy sutra
* Read sutra
* Practice generosity
* Refrain from killing
* Refrain from false speech
* Rejoice in merit

Possible visual style:

* Warm gold
* Deep red
* Lotus
* Incense smoke
* Temple lamp

## 6.3 Nichiren Pack

Tone:

* Chanting
* Lotus Sutra
* Daily practice
* Encouragement
* Human revolution / self-improvement framing, depending on user customization

Suggested labels:

* Practice
* Daimoku
* Gongyo
* Dedication
* Encouragement
* Vow
* Rejoice

Suggested practices:

* Chant daimoku
* Gongyo
* Read Lotus Sutra passage
* Encourage another person
* Support someone’s practice
* Study
* Share encouragement
* Refrain from slander
* Rejoice in another’s benefit

Default phrase support:

* “Namu Myōhō Renge Kyō”
* “Nam-myoho-renge-kyo”

Let user select romanization/spelling preference.

Possible visual style:

* Lotus
* Dawn gradient
* Scroll/paper
* Strong but not aggressive color palette

## 6.4 Pure Land Pack

Tone:

* Devotional
* Trust
* Gratitude
* Recitation
* Dedication

Suggested labels:

* Nembutsu
* Recitation
* Dedication
* Gratitude
* Rejoicing
* Vow

Suggested practices:

* Recite Amitabha/Amituofo/Amida
* Listen to Dharma talk
* Read Pure Land text
* Dedicate merit to all beings
* Practice gratitude
* Refrain from harsh speech
* Help another being
* Remember death compassionately
* Rejoice in another’s practice

Name options:

* Amitabha
* Amituofo
* Amida
* Namo Amituofo
* Namu Amida Butsu

User should choose preferred language/form.

Possible visual style:

* Sunset gold
* Lotus pond
* Soft light
* Blue/gold

## 6.5 Secular Pack

Tone:

* Non-theistic
* Ethical
* Reflective
* Habit-based

Drops or renames:

* Merit → Practice points
* Dedication of merit → Dedication / intention
* Repentance → Repair / reflection
* Precepts → Commitments
* Vows → Commitments
* Sentient beings → Living beings / others
* Rejoicing → Appreciating good

Disabled by default:

* Buddhas/bodhisattvas
* Sutra recitation templates
* Rebirth/lower realms language
* Ritual repentance language
* Chant-specific practices

Suggested practices:

* Acted generously
* Practiced patience
* Avoided harmful speech
* Helped someone
* Protected an animal
* Practiced mindful breathing
* Repaired a mistake
* Kept a commitment
* Rejoiced in someone else’s good action
* Reflected honestly

Possible visual style:

* Calm blue
* Green
* Natural light
* Journal-like

## 7. Practice Templates

A practice template defines something the user can record.

Fields:

```json
{
  "template_id": "pureland.nembutsu",
  "tradition": "pure_land",
  "name": "Nembutsu recitation",
  "description": "Record recitation practice.",
  "practice_type": "positive",
  "default_points": 10,
  "quantity_unit": "recitations",
  "default_quantity": 108,
  "allows_dedication": true,
  "allows_reflection": true,
  "visibility_default": "private",
  "tags": ["recitation", "devotion", "daily_practice"]
}
```

Practice types:

* positive
* negative_vow_breach
* repentance
* dedication
* rejoicing
* neutral_reflection

## 8. Vows

A vow is a user-defined or template-derived commitment.

Fields:

```json
{
  "vow_id": "vow_01HX...",
  "user_id": "local_user",
  "name": "Refrain from harsh speech",
  "description": "Practice restraint and kindness in speech.",
  "vow_type": "negative",
  "strength": "training_commitment",
  "status": "active",
  "frequency": "continuous",
  "start_date": "2026-07-05",
  "end_date": null,
  "default_points": 10,
  "repentance_category": "speech",
  "created_at": "2026-07-05T10:00:00Z",
  "updated_at": "2026-07-05T10:00:00Z"
}
```

Vow types:

* positive
* negative

Vow strength:

* aspiration
* training_commitment
* formal_vow
* experiment

Vow status:

* draft
* active
* paused
* repair_in_progress
* completed
* retired

## 9. Ledger Entries

All user activity becomes a ledger entry.

Entry types:

* practice_completed
* vow_completed
* vow_breached
* repentance_completed
* merit_dedicated
* mudita_rejoiced
* vow_created
* vow_paused
* vow_resumed
* vow_retired
* reflection

Fields:

```json
{
  "entry_id": "entry_01HX...",
  "user_id": "local_user",
  "entry_type": "practice_completed",
  "tradition": "pure_land",
  "template_id": "pureland.nembutsu",
  "vow_id": null,
  "title": "Nembutsu recitation",
  "occurred_at": "2026-07-05T09:00:00Z",
  "quantity": 108,
  "quantity_unit": "recitations",
  "points": 10,
  "dedication_id": "dedication_01HX...",
  "reflection": "Felt scattered but returned to practice.",
  "privacy": "private",
  "tags": ["recitation", "daily_practice"],
  "created_at": "2026-07-05T09:05:00Z",
  "updated_at": "2026-07-05T09:05:00Z"
}
```

Privacy values for MVP:

* private
* local_demo_visible

Future privacy values:

* group
* public
* federated

## 10. Repentance Entries

Repentance is implemented as a ledger entry with structured fields.

```json
{
  "entry_id": "entry_01HY...",
  "user_id": "local_user",
  "entry_type": "repentance_completed",
  "category": "speech",
  "title": "Returned to practice after harmful speech",
  "repentance_style": "category",
  "reflection": "I noticed impatience and will pause before replying.",
  "repair_intention": "Pause, breathe, and choose kinder speech.",
  "linked_vow_id": "vow_01HX...",
  "points": 5,
  "dedication_id": null,
  "occurred_at": "2026-07-05T14:00:00Z",
  "created_at": "2026-07-05T14:05:00Z"
}
```

Repentance categories:

* speech
* harm
* anger
* greed
* neglect
* intoxication
* carelessness
* broken_commitment
* general

## 11. Dedication Entries

Dedication can be linked to a practice entry or standalone.

```json
{
  "dedication_id": "dedication_01HZ...",
  "user_id": "local_user",
  "source_entry_id": "entry_01HX...",
  "target_type": "generic_group",
  "target_name": "All sentient beings",
  "dedication_text": "May this merit benefit all sentient beings.",
  "points_dedicated": 10,
  "created_at": "2026-07-05T09:06:00Z"
}
```

Target types:

* all_beings
* person
* animal
* ancestor
* family
* sick
* deceased
* generic_group
* custom

## 12. Scoring System

The scoring system must be customizable.

### 12.1 Default Scoring Modes

User chooses one:

1. Points mode
2. Count-only mode
3. Reflection-only mode

### 12.2 Point Rules

Each practice has:

* Default points
* Optional quantity multiplier
* Optional daily cap
* Optional diminishing returns
* Optional user override

Example:

```json
{
  "rule_id": "rule_zazen_minutes",
  "template_id": "zen.zazen",
  "base_points": 1,
  "quantity_multiplier": 1,
  "quantity_unit": "minutes",
  "daily_cap": 60,
  "allow_manual_override": true
}
```

### 12.3 Negative Vow Scoring

Default behavior:

* Breach creates repair/repentance flow
* No negative points by default
* Repentance may award practice points for returning to practice

Optional setting:

* Enable negative points

Default: off.

## 13. Local Data Architecture

The MVP should use a DynamoDB-compatible data modeling approach while avoiding the pain of requiring actual DynamoDB for a local desktop app.

### 13.1 Recommended MVP Storage

Use SQLite locally, but model it like a DynamoDB single-table design.

This gives:

* Easy desktop packaging
* No Java dependency
* No AWS account
* No local Docker requirement
* Future DynamoDB migration path
* Familiar single-table access patterns

### 13.2 Local Table

SQLite table:

```sql
CREATE TABLE merit_items (
    pk TEXT NOT NULL,
    sk TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    gsi1pk TEXT,
    gsi1sk TEXT,
    gsi2pk TEXT,
    gsi2sk TEXT,
    item_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (pk, sk)
);
```

This emulates a DynamoDB single-table layout.

### 13.3 Access Pattern Examples

User profile:

```text
PK = USER#local_user
SK = PROFILE
```

Ledger entry:

```text
PK = USER#local_user
SK = ENTRY#2026-07-05T09:00:00Z#entry_01HX
```

Vow:

```text
PK = USER#local_user
SK = VOW#vow_01HX
```

Dedication:

```text
PK = USER#local_user
SK = DEDICATION#2026-07-05T09:06:00Z#dedication_01HZ
```

Entries by date:

```text
PK = USER#local_user
SK begins_with ENTRY#2026-07
```

Vows by status:

```text
GSI1PK = USER#local_user#VOW_STATUS#active
GSI1SK = VOW#vow_01HX
```

Entries by type:

```text
GSI1PK = USER#local_user#ENTRY_TYPE#practice_completed
GSI1SK = ENTRY#2026-07-05T09:00:00Z#entry_01HX
```

Entries by tradition:

```text
GSI2PK = USER#local_user#TRADITION#pure_land
GSI2SK = ENTRY#2026-07-05T09:00:00Z#entry_01HX
```

### 13.4 Repository Interface

The domain should not know whether storage is SQLite or DynamoDB.

Define repository methods like:

```python
class MeritRepository:
    def put_item(self, item: MeritItem) -> None: ...
    def get_item(self, pk: str, sk: str) -> MeritItem | None: ...
    def query_pk(self, pk: str, sk_begins_with: str | None = None) -> list[MeritItem]: ...
    def query_gsi1(self, gsi1pk: str, gsi1sk_begins_with: str | None = None) -> list[MeritItem]: ...
    def query_gsi2(self, gsi2pk: str, gsi2sk_begins_with: str | None = None) -> list[MeritItem]: ...
    def delete_item(self, pk: str, sk: str) -> None: ...
```

Implementations:

* `SqliteMeritRepository` for MVP
* `DynamoDbMeritRepository` for paid/cloud
* `InMemoryMeritRepository` for tests

### 13.5 Why Not DynamoDB Local in MVP?

DynamoDB Local is useful for integration tests but awkward for a normal desktop app because it adds Java/runtime packaging complexity.

Use SQLite for the desktop MVP, but keep:

* Single-table shape
* PK/SK
* GSI-style columns
* JSON item payload
* Repository methods that mirror DynamoDB access patterns

This gives a clean cloud migration later.

## 14. Backend Architecture

The local backend is a FastAPI server started by the desktop app.

### 14.1 Process Model

Pygame launcher starts:

1. FastAPI backend on localhost
2. Pygame frontend
3. Frontend calls backend over HTTP

Default backend URL:

```text
http://127.0.0.1:8765
```

### 14.2 Backend Responsibilities

FastAPI handles:

* CRUD for ledger entries
* CRUD for vows
* CRUD for templates
* Tradition pack loading
* Scoring
* Repentance flows
* Dedication flows
* Export/import
* Local settings
* Local sample mudita feed

Pygame handles:

* Rendering
* Input
* Navigation
* Animations
* Local visual experience

### 14.3 API Endpoints

Health:

```http
GET /health
```

Profile:

```http
GET /profile
PUT /profile
```

Traditions:

```http
GET /traditions
GET /traditions/{tradition_id}
PUT /settings/tradition
```

Templates:

```http
GET /templates
GET /templates/{template_id}
POST /templates
PUT /templates/{template_id}
DELETE /templates/{template_id}
```

Ledger:

```http
GET /entries
GET /entries/{entry_id}
POST /entries
PUT /entries/{entry_id}
DELETE /entries/{entry_id}
```

Vows:

```http
GET /vows
GET /vows/{vow_id}
POST /vows
PUT /vows/{vow_id}
POST /vows/{vow_id}/pause
POST /vows/{vow_id}/resume
POST /vows/{vow_id}/retire
POST /vows/{vow_id}/complete
POST /vows/{vow_id}/breach
```

Repentance:

```http
GET /repentance/categories
POST /repentance
```

Dedication:

```http
GET /dedications
POST /dedications
```

Mudita demo:

```http
GET /mudita/demo-feed
POST /mudita/rejoice
```

Stats:

```http
GET /stats/today
GET /stats/week
GET /stats/month
GET /stats/by-template
GET /stats/by-tradition
GET /stats/vows
```

Export/import:

```http
GET /export/json
GET /export/markdown
POST /import/json
```

Settings:

```http
GET /settings
PUT /settings
```

## 15. Frontend Architecture

### 15.1 Pygame UI

The Pygame app should be built as a scene-based UI.

Scenes:

* SplashScene
* OnboardingScene
* DashboardScene
* RecordPracticeScene
* VowsScene
* VowDetailScene
* RepentanceScene
* DedicationScene
* MuditaGardenScene
* StatsScene
* SettingsScene
* ExportScene

### 15.2 Visual Style

The UI should feel like a calm shrine/journal hybrid.

Elements:

* Soft background gradients
* Rounded cards
* Gentle transitions
* Large readable text
* Tradition-specific theme palettes
* Simple iconography
* Subtle particles or light effects only if tasteful
* No flashing rewards
* No casino-style animations

### 15.3 Main Navigation

Main tabs:

1. Today
2. Record
3. Vows
4. Repent
5. Dedicate
6. Mudita
7. Stats
8. Settings

### 15.4 Dashboard

Dashboard shows:

* Today’s practice total
* Active vows
* Suggested practice
* Quick buttons
* Recent ledger entries
* Dedication reminder
* Gentle quote/instruction from selected tradition pack

Example dashboard cards:

* “Return to practice”
* “Record merit”
* “Today’s vows”
* “Dedicate”
* “Rejoice”

### 15.5 Pretty MVP Details

Small touches:

* When recording merit, a lotus/card gently appears in the ledger.
* When dedicating merit, points flow into a soft light animation.
* When rejoicing, a small flower appears in the Mudita Garden.
* When completing repentance, the UI says: “Returned to practice.”
* Paused vows become dimmed but not red.
* Broken vows show “repair available,” not failure.

## 16. Mudita Garden

The Mudita Garden is the MVP’s social preview.

It is local-only.

It contains generated/sample entries from tradition packs.

Example UI:

```text
Someone practiced patience today.
[Rejoice]

Someone dedicated chanting to all beings.
[Rejoice]

Someone returned to practice after difficulty.
[Rejoice]
```

When user rejoices:

* Add flower to garden
* Create local ledger entry
* Award configurable points
* No network call

Future cloud version can replace this with real group-visible practice entries.

## 17. Settings

Settings categories:

### 17.1 Practice Mode

* Tradition
* Mixed mode
* Secular mode
* Label style

### 17.2 Points

* Points enabled
* Count-only
* Reflection-only
* Negative points enabled
* Daily caps
* Diminishing returns

### 17.3 Privacy

* Local-only notice
* Hide reflections in stats
* Require confirmation before export
* Repentance safety reminders

### 17.4 Appearance

* Theme
* Font size
* Reduced motion
* High contrast
* Sound on/off

### 17.5 Data

* Export JSON
* Export Markdown
* Import JSON
* Clear local data
* Backup location

## 18. Secular Mode Behavior

When secular mode is enabled:

Rename:

* Merit → Practice points
* Vow → Commitment
* Repentance → Repair
* Dedication → Intention
* Rejoicing/Mudita → Appreciating good
* Precepts → Ethical commitments

Hide or disable:

* Buddha/bodhisattva names
* Sutra recitation defaults
* Rebirth language
* Ritual repentance language
* “All sentient beings” unless user enables Buddhist language

Replace dedication examples:

* “May this merit benefit all sentient beings.”

With:

* “May this action support the well-being of others.”
* “I dedicate this effort to becoming more patient and helpful.”
* “May this practice make me more useful to those around me.”

## 19. Data Export

### 19.1 JSON Export

Should include:

* Profile
* Settings
* Tradition selection
* Custom templates
* Vows
* Ledger entries
* Dedications
* Repentance entries

### 19.2 Markdown Export

Human-readable export:

```markdown
# Merit Ledger Export

## Profile

Mode: Pure Land
Points enabled: true

## Recent Entries

### 2026-07-05 - Nembutsu recitation

Quantity: 108 recitations  
Points: 10  
Dedication: All sentient beings

Reflection:
Felt scattered but returned to practice.
```

### 19.3 Privacy Warning

Before export:

> Exported files may contain private reflections. Review before sharing.

## 20. File Layout

Recommended project layout:

```text
merit-ledger/
  pyproject.toml
  README.md
  LICENSE
  src/
    merit_ledger/
      __init__.py
      app.py
      backend/
        __init__.py
        main.py
        api/
          health.py
          profile.py
          traditions.py
          templates.py
          entries.py
          vows.py
          repentance.py
          dedication.py
          mudita.py
          stats.py
          settings.py
          export.py
        domain/
          models.py
          scoring.py
          vows.py
          repentance.py
          dedication.py
          stats.py
        repository/
          base.py
          sqlite_repo.py
          memory_repo.py
          dynamodb_repo.py
          item_keys.py
        services/
          entry_service.py
          vow_service.py
          repentance_service.py
          dedication_service.py
          stats_service.py
          export_service.py
        tradition_packs/
          zen.json
          chinese_mahayana.json
          nichiren.json
          pure_land.json
          secular.json
      frontend/
        __init__.py
        pygame_app.py
        scenes/
          splash.py
          onboarding.py
          dashboard.py
          record_practice.py
          vows.py
          vow_detail.py
          repentance.py
          dedication.py
          mudita_garden.py
          stats.py
          settings.py
        ui/
          buttons.py
          cards.py
          layout.py
          theme.py
          text.py
          animation.py
        assets/
          fonts/
          icons/
          sounds/
          themes/
      local/
        data_dir.py
        server_process.py
        config.py
  tests/
    test_scoring.py
    test_vows.py
    test_repentance.py
    test_dedication.py
    test_sqlite_repo.py
    test_api_entries.py
    test_api_vows.py
```

## 21. Suggested Dependencies

Runtime:

```text
fastapi
uvicorn
pydantic
pygame-ce
httpx
platformdirs
```

Storage:

```text
sqlite3 from stdlib
```

Testing:

```text
pytest
pytest-cov
hypothesis
mypy
ruff
```

Optional future/cloud:

```text
boto3
```

Recommended package manager:

```text
uv
```

## 22. Local App Startup

Command:

```bash
merit-ledger
```

Startup behavior:

1. Determine local data directory using `platformdirs`
2. Create SQLite database if needed
3. Start FastAPI backend on localhost
4. Wait for `/health`
5. Launch Pygame frontend
6. On app close, shut down backend

Local data directory examples:

Windows:

```text
%LOCALAPPDATA%/MeritLedger/
```

macOS:

```text
~/Library/Application Support/MeritLedger/
```

Linux:

```text
~/.local/share/merit-ledger/
```

## 23. Testing Strategy

### 23.1 Unit Tests

Cover:

* Scoring
* Vow state transitions
* Repentance creation
* Dedication creation
* Tradition pack loading
* Key generation
* Export/import

### 23.2 Repository Tests

Every repository implementation should pass the same contract tests.

Repositories:

* InMemoryMeritRepository
* SqliteMeritRepository
* DynamoDbMeritRepository later

### 23.3 API Tests

Use FastAPI test client.

Cover:

* Create entry
* List entries
* Create vow
* Pause vow
* Resume vow
* Complete vow
* Breach vow
* Create repentance
* Create dedication
* Export JSON
* Import JSON

### 23.4 Frontend Tests

Pygame is harder to test. Keep business logic out of frontend.

Test:

* Scene state transitions where practical
* API client behavior
* Theme loading
* View models

## 24. MVP Build Phases

## Phase 1: Backend Skeleton

Deliver:

* FastAPI app
* Health endpoint
* SQLite single-table repository
* Pydantic models
* Tradition pack loading
* Basic tests

## Phase 2: Ledger

Deliver:

* Create/list/update/delete ledger entries
* Practice templates
* Scoring
* JSON export

## Phase 3: Vows

Deliver:

* Create vow
* Pause/resume vow
* Complete positive vow
* Breach negative vow
* Repair status

## Phase 4: Repentance and Dedication

Deliver:

* Repentance categories
* Omnibus repentance
* Linked repentance for broken vows
* Dedication targets
* Dedication presets

## Phase 5: Pygame Shell

Deliver:

* Launch app
* Backend process manager
* Splash screen
* Onboarding
* Dashboard
* Navigation

## Phase 6: Pygame Core Screens

Deliver:

* Record practice
* Vows
* Vow detail
* Repentance
* Dedication
* Stats
* Settings

## Phase 7: Beauty Pass

Deliver:

* Tradition themes
* Card UI
* Animations
* Mudita Garden
* Better typography
* Reduced motion setting

## Phase 8: Packaging

Deliver:

* CLI entry point
* Local data directory
* README
* Screenshots
* Basic installer notes
* GitHub release artifact

## 25. MVP Acceptance Criteria

The MVP is done when:

1. User can install and launch local desktop app.
2. User can select Zen, Chinese Mahayana, Nichiren, Pure Land, Secular, or Custom/Mixed mode.
3. User can record positive practices.
4. User can create positive and negative vows.
5. User can pause and resume vows.
6. User can record completion of positive vows.
7. User can record breach of negative vows.
8. Breach can trigger repentance/repair flow.
9. User can dedicate merit/practice points.
10. User can rejoice in local Mudita Garden entries.
11. User can customize point values.
12. User can view daily/weekly/monthly stats.
13. User can export JSON.
14. User can export Markdown.
15. User can import JSON backup.
16. App stores data locally.
17. Storage uses DynamoDB-compatible single-table key structure.
18. Business logic is independent of SQLite/Pygame.
19. App contains privacy warnings around repentance and export.
20. UI is visually pleasant enough to screenshot proudly.

## 26. Future Cloud Migration

The paid cloud version should reuse:

* Domain models
* Repository interface
* Single-table key design
* API shapes
* Tradition packs
* Scoring logic
* Export/import logic

Cloud additions:

* Auth
* Real user IDs
* DynamoDB table
* S3 export storage
* API Gateway / Lambda or ECS
* Web UI
* Mobile UI
* Sync conflict strategy
* API keys
* Billing
* Groups
* Real mudita feed
* Public/private visibility
* Optional federation

## 27. Cloud Data Model Compatibility

Future real DynamoDB table:

```text
Table: MeritLedgerItems

PK: pk
SK: sk

GSI1:
  PK: gsi1pk
  SK: gsi1sk

GSI2:
  PK: gsi2pk
  SK: gsi2sk
```

Local SQLite columns should map directly to these fields.

This allows migration:

1. Export local JSON
2. Transform each item to DynamoDB PutItem shape
3. Upload to cloud account
4. Continue using same access patterns

## 28. Anti-Features

Do not add these to MVP:

* Public leaderboards
* Streak shame
* Confession textbox that asks for details
* AI judgment of morality
* “You lost karma”
* Competitive rankings
* Public default visibility
* Ads
* Infinite feed
* Algorithmic engagement
* Dark patterns
* Pushy daily guilt notifications

## 29. Possible App Names

Working name:

* Merit Ledger

Other possible names:

* Mudita Garden
* Merit Garden
* Practice Ledger
* Vow Garden
* Dedication Ledger
* Return to Practice
* Lotus Ledger
* Bodhi Ledger
* Good Roots
* Field of Merit

## 30. MVP Tagline Options

* “A private ledger for vows, merit, repentance, and dedication.”
* “Track practice. Repair gently. Dedicate the good.”
* “Less leaderboard, more liberation.”
* “A quiet place to record wholesome practice.”
* “Private-first Buddhist practice tracking.”
* “A mudita-centered practice journal.”

## 31. Recommended MVP Positioning

Best MVP positioning:

> Merit Ledger is a private-first Buddhist practice journal for recording merit, vows, repentance, dedication, and rejoicing. It supports Zen, Chinese Mahayana, Nichiren, Pure Land, and secular ethical practice modes.

## 32. First Build Recommendation

Start with the backend.

The first technical milestone should be:

* `POST /entries`
* `GET /entries`
* `POST /vows`
* `POST /vows/{id}/complete`
* `POST /vows/{id}/breach`
* SQLite single-table repository
* Pure Land, Zen, Nichiren, Chinese Mahayana, and Secular JSON packs

Then build a very simple Pygame dashboard.

Do not start with animations.
Do not start with cloud.
Do not start with federation.
Do not start with social.

Start with the sacred boring thing:

> Can I record practice, keep vows, repair breaches, and dedicate merit locally?

Once that works, make it beautiful.
