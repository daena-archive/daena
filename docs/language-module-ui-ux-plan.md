# Language Module UI/UX Improvement Plan

## Implementation Status

### Phase 1: Navigation & Information Architecture ✅ COMPLETED

**Completed Items:**

1. **Tab Grouping with Clearer Labels**
   - Reorganized 7 flat tabs into 3 logical groups:
     - Foundation: Overview, Lexicon
     - Phonology & Writing: Sounds, Writing
     - Grammar & Structure: Grammar, Morphology (renamed from "Forms"), Samples
   - Added group labels above each tab group for better visual hierarchy

2. **Breadcrumb Navigation**
   - Added breadcrumb showing: Language Studio > Language Name > Current Section
   - Provides clear context for where the user is in the module

3. **Improved Sidebar Organization**
   - Added language count indicator showing total number of languages
   - Improved search input with placeholder text
   - Better visual hierarchy for sidebar elements

4. **Status Indicators for Languages**
   - Added "Archived" status badge for deleted languages
   - Improved language list layout with flexbox for better alignment

5. **Clear Filter Indicators**
   - Added filter status showing "Showing X of Y languages" when filtering
   - Added "Clear" button to quickly reset filters

6. **Streamlined Create Language Form**
   - Added header with close button
   - Added placeholder text for input
   - Disabled create button when name is empty
   - Improved visual hierarchy and spacing

**Files Modified:**
- `packages/modules/language/src/LanguageWorkspace.svelte`
- `packages/modules/language/src/panes/Overview.svelte`

### Phase 2: Visual Hierarchy & Layout ✅ COMPLETED

**Completed Items:**

1. **Overview Pane Redesign**
   - Improved identity section with better header layout
   - Archive button moved to header for better accessibility
   - Properties section with improved field grid layout
   - Better responsive design for mobile screens
   - Consistent section styling with headers

2. **Lexicon Filters with Progressive Disclosure**
   - Improved search row with filter badge showing active filter count
   - Better filter panel with summary showing filter count badge
   - Improved filter layout with placeholder text
   - Better visual feedback for active filters
   - Clear all filters button only shows when filters are active

3. **Sounds Pane Beginner Mode**
   - Added beginner mode toggle to toolbar
   - Simplified phoneme editor in beginner mode (hides advanced features)
   - Organized editor into logical sections (Basic info, Consonant features, Vowel features, Additional details)
   - Added placeholder text for better guidance
   - Advanced features hidden by default, accessible via toggle

4. **Visual Hierarchy Enhancements**
   - Added global CSS for form sections with consistent styling
   - Added global section grid layout for consistent 2-column layouts
   - Added global toolbar styles for consistency across panes
   - Added global action button styles
   - Improved responsive design with mobile-first approach
   - Better spacing and typography across all panes

**Files Modified:**
- `packages/modules/language/src/LanguageWorkspace.svelte`
- `packages/modules/language/src/panes/Overview.svelte`
- `packages/modules/language/src/panes/Lexicon.svelte`
- `packages/modules/language/src/panes/Sounds.svelte`

### Phase 3: Interaction Patterns & Workflows ✅ COMPLETED

**Completed Items:**

1. **Grammar Pane Navigation with Progress**
   - Added progress indicators to section cards showing completion percentage
   - Added progress bars that fill based on configured systems
   - Added percentage display for each section
   - Improved visual hierarchy with better card layout

2. **Forms Pane Visual Builder**
   - Improved slots section with visual card grid layout
   - Added slot numbers and remove buttons
   - Improved rules section with collapsible details
   - Added rule kind badges
   - Improved operations section with better layout
   - Added placeholder text for better guidance

3. **Samples Pane Token Editor**
   - Improved token editor with table layout
   - Added column headers for better organization
   - Added token numbers for reference
   - Improved mobile responsive design
   - Better visual feedback for token operations

4. **Better Preview Functionality**
   - Improved preview section with better styling
   - Added descriptive header and subtitle
   - Better visual hierarchy for preview content
   - Improved responsive design

**Files Modified:**
- `packages/modules/language/src/panes/Grammar.svelte`
- `packages/modules/language/src/panes/Forms.svelte`
- `packages/modules/language/src/panes/Samples.svelte`

### Phase 5.1: Onboarding Flow ✅ COMPLETED

**Completed Items:**

1. **Welcome Tour Component** (`WelcomeTour.svelte`)
   - Created step-by-step tour with 7 steps covering key features
   - Progress bar and dot navigation
   - Keyboard navigation support (Escape, Arrow keys, Enter)
   - Responsive design with backdrop blur
   - LocalStorage persistence to remember completion

2. **Contextual Help Tooltips** (`HelpTooltip.svelte`)
   - Created reusable tooltip component
   - Supports 4 positions (top, bottom, left, right)
   - Auto-shows on hover and focus
   - Accessible with ARIA attributes
   - Arrow pointing to trigger element

3. **Language Templates** (`templates.ts`)
   - Created 5 language templates:
     - Minimal Language - basic starting point
     - Naturalistic Language - inspired by real languages
     - Artistic Language - focused on aesthetics
     - Engineered Language - logical and precise
     - Proto-Language - starting point for language families
   - Each template includes fields and optional starter systems

4. **Integration into LanguageWorkspace**
   - Added welcome tour on first visit (localStorage check)
   - Added "Show tour" button in sidebar
   - Added template selection in create language flow
   - Templates apply predefined fields on creation
   - Template name shown in create form

**Files Modified:**
- `packages/modules/language/src/WelcomeTour.svelte` (new)
- `packages/modules/language/src/HelpTooltip.svelte` (new)
- `packages/modules/language/src/templates.ts` (new)
- `packages/modules/language/src/LanguageWorkspace.svelte`

---

## Current State Analysis

### Strengths

- Well-structured pane system with 7 logical sections (Overview, Lexicon, Sounds, Writing, Grammar, Forms, Samples)
- Responsive design with mobile-friendly layout
- Good accessibility patterns (ARIA labels, keyboard navigation)
- Consistent design language with CSS variables
- Clear status feedback for saving states

### Key Issues Identified

#### 1. Navigation & Information Architecture

- **7 tabs** can be overwhelming for new users
- No visual hierarchy or grouping of related features
- Tab labels could be more descriptive (e.g., "Forms" -> "Morphology")
- No breadcrumb or progress indication for complex workflows

#### 2. Sidebar Design

- "Language studio" kicker text may confuse users
- Filter input placement could be more prominent
- No language count or status indicators
- Create language form could be more streamlined

#### 3. Pane-Specific Issues

**Overview Pane:**

- Two-column layout cramped on smaller screens
- Archive button placement unclear
- Status indicator could be more prominent
- Field organization could be improved

**Lexicon Pane:**

- Complex filter system (4+ filters) overwhelming
- Editor panel organization could be simplified
- Import/export functionality hidden
- Paradigm integration unclear

**Sounds Pane:**

- IPA chart intimidating for non-linguists
- Phoneme editor complex for beginners
- Phonology notes section buried
- Chart interaction could be improved

**Writing Pane:**

- Orthography mapping complex
- Multiple writing systems management unclear
- Sound mapping interface could be simplified

**Grammar Pane:**

- Complex section navigation (8+ sections)
- Multiple editor types confusing
- Starter system guidance could be improved
- Search and filtering overwhelming

**Forms Pane:**

- Paradigm editing complex
- Preview functionality underutilized
- Rule management could be simplified

**Samples Pane:**

- Token editor unintuitive
- Preview rendering basic
- Lexeme linking unclear

#### 4. General UX Issues

- Inconsistent button styles across panes
- Complex state management visible to users
- Error handling could be more user-friendly
- Loading states could be more informative
- No onboarding or help system

---

## Improvement Plan

### Phase 1: Navigation & Information Architecture (High Priority)

#### 1.1 Tab Reorganization & Grouping

Current flat tabs:

```text
["overview", "lexicon", "sounds", "writing", "grammar", "forms", "samples"]
```

Proposed grouped tabs with clearer labels:

```text
Foundation
  - Overview
  - Lexicon

Phonology & Writing
  - Sounds
  - Writing

Grammar & Structure
  - Grammar
  - Morphology (renamed from "Forms")
  - Samples
```

**Benefits:**

- Reduces cognitive load by grouping related features
- Clearer mental model for users
- Better discoverability of related features

#### 1.2 Breadcrumb Navigation

Add breadcrumb showing:

```text
Language Name > Current Section > Subsection
```

**Features:**

- Enable quick navigation between related sections
- Show progress for complex workflows (e.g., "Creating new paradigm")
- Provide context for where user is in the module

#### 1.3 Sidebar Improvements

Enhanced sidebar with better organization:

- **Search & Filters:**
  - More prominent search input
  - Collapsible advanced filters
  - Clear filter indicators

- **Language List:**
  - Show language count
  - Status indicators (active, draft, archived)
  - Visual language cards with key info

- **Create Language:**
  - Streamlined single-step form
  - Template suggestions
  - Quick-start options

---

### Phase 2: Visual Hierarchy & Layout (Medium Priority)

#### 2.1 Overview Pane Redesign

**Layout Improvements:**

- Single column on mobile, two columns on desktop
- Grouped fields with clear sections:
  - **Identity:** Name, native name
  - **Classification:** Family, status
  - **Writing System:** Writing system type
  - **Notes:** Description, canonical notes

- Sticky save status indicator
- Improved archive placement (sidebar or footer)

**Visual Improvements:**

- Clear section headers
- Better spacing between field groups
- Visual hierarchy for primary vs. secondary fields

#### 2.2 Lexicon Pane Improvements

**Simplified Filter System:**

```text
Primary Filter (always visible):
  - Search input
  - Part of speech dropdown

Secondary Filters (collapsible):
  - Status filter
  - Tags filter
  - Homonyms only toggle
  - Sort options (lemma, modified, created)
```

**View Modes:**

- List view (current)
- Table view (for bulk editing)
- Card view (for visual browsing)

**Quick Actions:**

- Import/export buttons more visible
- Bulk edit functionality
- Quick-add word button

#### 2.3 Sounds Pane Redesign

**Progressive Disclosure:**

```text
Beginner Mode (default):
  - Simplified phoneme editor
  - Basic IPA chart (consonants/vowels)
  - Hide advanced features

Advanced Mode (toggle):
  - Full phoneme editor with features
  - Complete IPA chart with all symbols
  - Phonotactics and phonology notes
```

**Chart Improvements:**

- Highlight on hover
- Show empty cells option
- Group by features (place, manner, voicing)
- Click-to-add functionality

**Editor Improvements:**

- Show IPA input hints
- Feature suggestions based on symbol
- Example words for each phoneme

#### 2.4 Writing Pane Improvements

- Simplified orthography mapping interface
- Better management of multiple writing systems
- Visual grapheme-to-sound mapping
- Preview functionality for writing systems

---

### Phase 3: Interaction Patterns & Workflows (Medium Priority)

#### 3.1 Grammar Pane Improvements

**Section Navigation with Progress:**

```text
Word Order [progress bar]
  - Basic word order
  - Question formation

Nouns [progress bar]
  - Noun classes
  - Pluralization

Verbs [progress bar]
  - Tense/aspect
  - Verb conjugation
```

**Starter System Guidance:**

- Show progress indicator
- Show next recommended step
- Estimated time to complete
- Skip option for advanced users

**Editor Improvements:**

- Show preview pane
- Real-time validation
- Auto-save functionality
- Conflict detection

#### 3.2 Forms Pane Improvements

**Visual Paradigm Builder:**

- Drag-and-drop slot arrangement
- Template paradigms (regular verbs, nouns, etc.)
- Visual preview of generated forms

**Rule Management:**

- Visual rule editor
- Rule preview showing affected words
- Conflict detection between rules
- Rule ordering interface

**Preview Improvements:**

- Show multiple lexemes at once
- Compare generated vs. authored forms
- Export paradigm tables

#### 3.3 Samples Pane Improvements

**Token Editor:**

- Inline editing of tokens
- Auto-complete for lexeme linking
- Validation of glosses and grammar tags
- Visual token boundaries

**Preview Improvements:**

- Interlinear glossing display
- Translation alignment
- Grammar annotation highlights
- Lexeme link visualization

**Linking Improvements:**

- Lexeme suggestion dropdown
- Grammar tag auto-complete
- Auto-linking based on token text
- Bulk linking tools

---

### Phase 4: Accessibility & Responsive Design (Medium Priority)

#### 4.1 Mobile-First Improvements

**Bottom Navigation:**

```text
[Overview] [Lexicon] [Sounds] [Writing] [Grammar]
```

**Touch Gestures:**

- Swipe between panes
- Swipe to edit/delete items
- Long-press for context menus

**Touch-Friendly Controls:**

- Larger buttons (44px minimum)
- Better spacing between interactive elements
- Haptic feedback for key actions

#### 4.2 Accessibility Enhancements

**Screen Reader Improvements:**

- Better ARIA labels for complex controls
- Live regions for status updates
- Skip links for navigation
- Descriptive error messages

**Keyboard Navigation:**

- Focus management for modal dialogs
- Keyboard shortcuts for common actions
- Roving tabindex for tab groups
- Escape key to cancel/close

**Visual Accessibility:**

- High contrast mode
- Reduced motion option
- Scalable text (up to 200%)
- Color-blind friendly palette

---

### Phase 5: Onboarding & Help System (Low Priority)

#### 5.1 Onboarding Flow

**Welcome Tour:**

- Step-by-step introduction to key features
- Interactive tooltips
- Progress indicator
- Skip option

**Contextual Help:**

- Tooltips on hover/focus
- Inline help text
- "What's this?" mode

**Templates:**

- Pre-built language templates
- Quick-start wizard
- Example languages to explore
- Import from common formats

#### 5.2 Help System

**Documentation:**

- Inline documentation
- Searchable help articles
- Examples and use cases
- Glossary of linguistic terms

**Community:**

- Language showcase gallery
- User forums/discussions
- Feature request system
- Bug reporting

**Support:**

- Contact form
- FAQ section
- Troubleshooting guides
- Video walkthroughs

---

## Implementation Roadmap

### Phase 1 (Weeks 1-2): Navigation & Information Architecture

- [x] Implement tab grouping with clear labels
- [x] Add breadcrumb navigation
- [x] Improve sidebar with better organization
- [x] Add language count and status indicators
- [x] Streamline create language form

### Phase 2 (Weeks 3-4): Visual Hierarchy & Layout

- [x] Redesign overview pane with responsive grid
- [x] Simplify lexicon filters with progressive disclosure
- [x] Improve sounds pane with beginner mode
- [x] Enhance visual hierarchy across all panes

### Phase 3 (Weeks 5-6): Interaction Patterns & Workflows

- [x] Improve grammar pane navigation with progress
- [x] Enhance forms pane with visual builder
- [x] Improve samples pane token editor
- [x] Add better preview functionality

### Phase 4 (Weeks 7-8): Accessibility & Responsive Design

- [ ] Implement mobile-first navigation
- [ ] Add touch-friendly controls
- [ ] Enhance screen reader support
- [ ] Improve keyboard navigation

### Phase 5 (Weeks 9-10): Onboarding & Help System

- [x] Implement welcome tour
- [x] Add contextual help
- [x] Create language templates
- [ ] Build help documentation

---

## Success Metrics

### User Engagement

- Time spent in language module
- Number of languages created
- Feature adoption rates
- Return user rate

### Usability

- Task completion rates
- Error rates
- User satisfaction scores
- Time on task

### Accessibility

- WCAG 2.1 AA compliance
- Screen reader compatibility
- Keyboard navigation coverage
- Color contrast ratios

### Performance

- Load times (< 2s initial load)
- Interaction responsiveness (< 100ms)
- Memory usage
- Bundle size

---

## Questions for Clarification

1. **Priority:** Which phase should be prioritized based on user feedback?
2. **Scope:** Are there specific panes that need more attention than others?
3. **Constraints:** Are there any technical constraints or limitations to consider?
4. **Timeline:** Is the 10-week timeline realistic for the proposed improvements?
5. **Resources:** What resources are available for implementation?

---

## Appendix: Technical Considerations

### Component Architecture

- Maintain Svelte 5 patterns with runes
- Keep component props interface clean
- Use CSS variables for theming
- Preserve existing accessibility patterns

### State Management

- Keep state local to components where possible
- Use derived state for computed values
- Implement proper cleanup in effects
- Maintain leave-guard patterns

### Performance

- Lazy load heavy components (IPA chart, paradigm preview)
- Debounce user input (search, filters)
- Virtualize long lists (lexicon, samples)
- Cache computed values

### Testing

- Unit tests for utility functions
- Component tests for UI interactions
- Integration tests for workflows
- Accessibility audits

---

*Document created: 2026-08-18*
*Last updated: 2026-08-18*
