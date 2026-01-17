/**
 * RSK Skill Ecosystem Architecture Wireframe
 *
 * Interactive React component visualizing the Rust/Python skill ecosystem.
 * Requires: React 18+, Tailwind CSS, Lucide React icons
 *
 * Usage: <RSKArchitectureWireframe />
 */

import React, { useState } from 'react';
import {
  Layers, Database, Cpu, Code, Box, ArrowRight,
  CheckCircle, AlertTriangle, Zap, Terminal, FileCode,
  GitBranch, Package, Shield, Activity, Server
} from 'lucide-react';

const RSKArchitectureWireframe = () => {
  const [activeLayer, setActiveLayer] = useState('all');
  const [activeModule, setActiveModule] = useState(null);

  // Layer filter buttons
  const layers = [
    { id: 'all', label: 'All Layers', icon: Layers },
    { id: 'rust', label: 'Rust Kernel', icon: Cpu },
    { id: 'bridge', label: 'Python Bridge', icon: Code },
    { id: 'skills', label: 'Skill Framework', icon: Box },
  ];

  // RSK Modules with metadata
  const rskModules = [
    { name: 'python_bindings', size: '45KB', functions: 34, criticality: 'CRITICAL', color: 'red' },
    { name: 'text_processor', size: '40KB', functions: 8, criticality: 'HIGH', color: 'orange' },
    { name: 'code_generator', size: '30KB', functions: 6, criticality: 'HIGH', color: 'orange' },
    { name: 'execution_engine', size: '28KB', functions: 5, criticality: 'HIGH', color: 'orange' },
    { name: 'graph', size: '27KB', functions: 6, criticality: 'HIGH', color: 'orange' },
    { name: 'routing_engine', size: '26KB', functions: 4, criticality: 'MEDIUM', color: 'yellow' },
    { name: 'state_manager', size: '25KB', functions: 9, criticality: 'HIGH', color: 'orange' },
    { name: 'yaml_processor', size: '25KB', functions: 7, criticality: 'HIGH', color: 'orange' },
    { name: 'taxonomy', size: '23KB', functions: 4, criticality: 'HIGH', color: 'orange' },
    { name: 'telemetry', size: '14KB', functions: 5, criticality: 'MEDIUM', color: 'yellow' },
    { name: 'levenshtein', size: '12KB', functions: 3, criticality: 'MEDIUM', color: 'yellow' },
    { name: 'crypto', size: '8KB', functions: 3, criticality: 'MEDIUM', color: 'yellow' },
    { name: 'compression', size: '6KB', functions: 4, criticality: 'LOW', color: 'green' },
    { name: 'math', size: '5KB', functions: 2, criticality: 'LOW', color: 'green' },
  ];

  // Performance benchmarks
  const benchmarks = [
    { op: 'Levenshtein (short)', python: '0.8ms', rust: '0.04ms', speedup: '18.8x' },
    { op: 'Levenshtein (long)', python: '69ms', rust: '1ms', speedup: '69.1x' },
    { op: 'Fuzzy search', python: '38ms', rust: '1.2ms', speedup: '32x' },
    { op: 'YAML parse', python: '5.7μs', rust: '0.2μs', speedup: '29x' },
    { op: 'SMST extract', python: '50ms', rust: '3.3ms', speedup: '15x' },
    { op: 'Taxonomy lookup', python: '3.4μs', rust: 'O(1)', speedup: '∞' },
  ];

  // Compliance levels
  const complianceLevels = [
    { level: 'Bronze', badge: '🥉', count: 229, color: 'amber' },
    { level: 'Silver', badge: '🥈', count: 180, color: 'slate' },
    { level: 'Gold', badge: '🥇', count: 100, color: 'yellow' },
    { level: 'Platinum', badge: '💎', count: 50, color: 'cyan' },
    { level: 'Diamond', badge: '💠', count: 20, color: 'blue' },
  ];

  // Data flow stages
  const dataFlow = [
    { stage: 'Request', icon: Terminal, desc: 'User invokes skill via CLI/Slash command' },
    { stage: 'Route', icon: GitBranch, desc: 'Skill router matches pattern to SKILL.md' },
    { stage: 'Bridge', icon: Code, desc: 'rsk_bridge.py determines backend (PyO3/CLI/Python)' },
    { stage: 'Execute', icon: Zap, desc: 'RSK kernel processes with 10-70x speedup' },
    { stage: 'Return', icon: Package, desc: 'TypedDict result returned to skill' },
  ];

  const CriticalityBadge = ({ level }) => {
    const colors = {
      CRITICAL: 'bg-red-600',
      HIGH: 'bg-orange-500',
      MEDIUM: 'bg-yellow-500',
      LOW: 'bg-green-500',
    };
    return (
      <span className={`${colors[level]} text-white text-xs px-2 py-0.5 rounded`}>
        {level}
      </span>
    );
  };

  return (
    <div className="min-h-screen bg-gray-900 text-gray-100 p-6">
      {/* Header */}
      <div className="mb-8">
        <h1 className="text-3xl font-bold mb-2 flex items-center gap-3">
          <Cpu className="w-8 h-8 text-orange-500" />
          RSK Skill Ecosystem Architecture
        </h1>
        <p className="text-gray-400">
          v0.5.0 | 34 PyO3 Functions | 229 Skills | 265 Tests Passing
        </p>
      </div>

      {/* Layer Filters */}
      <div className="flex gap-2 mb-6">
        {layers.map(layer => (
          <button
            key={layer.id}
            onClick={() => setActiveLayer(layer.id)}
            className={`flex items-center gap-2 px-4 py-2 rounded-lg transition-colors ${
              activeLayer === layer.id
                ? 'bg-orange-600 text-white'
                : 'bg-gray-800 text-gray-300 hover:bg-gray-700'
            }`}
          >
            <layer.icon className="w-4 h-4" />
            {layer.label}
          </button>
        ))}
      </div>

      <div className="grid grid-cols-12 gap-6">
        {/* Section 1: RSK Modules Grid */}
        {(activeLayer === 'all' || activeLayer === 'rust') && (
          <div className="col-span-8 bg-gray-800 rounded-xl p-6">
            <h2 className="text-xl font-semibold mb-4 flex items-center gap-2">
              <Cpu className="w-5 h-5 text-orange-500" />
              Rust Kernel Modules (14)
            </h2>
            <div className="grid grid-cols-4 gap-3">
              {rskModules.map(mod => (
                <div
                  key={mod.name}
                  onClick={() => setActiveModule(activeModule === mod.name ? null : mod.name)}
                  className={`p-3 rounded-lg cursor-pointer transition-all ${
                    activeModule === mod.name
                      ? 'bg-orange-600 ring-2 ring-orange-400'
                      : 'bg-gray-700 hover:bg-gray-600'
                  }`}
                >
                  <div className="font-mono text-sm truncate">{mod.name}</div>
                  <div className="flex justify-between items-center mt-2">
                    <span className="text-xs text-gray-400">{mod.size}</span>
                    <span className="text-xs text-orange-400">{mod.functions} fn</span>
                  </div>
                  <div className="mt-2">
                    <CriticalityBadge level={mod.criticality} />
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Section 2: Performance Benchmarks */}
        {(activeLayer === 'all' || activeLayer === 'rust') && (
          <div className="col-span-4 bg-gray-800 rounded-xl p-6">
            <h2 className="text-xl font-semibold mb-4 flex items-center gap-2">
              <Zap className="w-5 h-5 text-yellow-500" />
              Performance
            </h2>
            <div className="space-y-3">
              {benchmarks.map(b => (
                <div key={b.op} className="bg-gray-700 rounded-lg p-3">
                  <div className="text-sm font-medium">{b.op}</div>
                  <div className="flex justify-between mt-1 text-xs">
                    <span className="text-red-400">Python: {b.python}</span>
                    <span className="text-green-400">Rust: {b.rust}</span>
                    <span className="text-orange-400 font-bold">{b.speedup}</span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Section 3: Python Bridge Layer */}
        {(activeLayer === 'all' || activeLayer === 'bridge') && (
          <div className="col-span-6 bg-gray-800 rounded-xl p-6">
            <h2 className="text-xl font-semibold mb-4 flex items-center gap-2">
              <Code className="w-5 h-5 text-blue-500" />
              Python Bridge Layer
            </h2>
            <div className="space-y-4">
              {/* Priority Chain */}
              <div className="bg-gray-700 rounded-lg p-4">
                <h3 className="font-medium mb-3">Function Resolution Chain</h3>
                <div className="flex items-center gap-2 text-sm">
                  <div className="bg-green-600 px-3 py-2 rounded">
                    PyO3 Direct<br/>
                    <span className="text-xs opacity-75">11-70x speedup</span>
                  </div>
                  <ArrowRight className="w-4 h-4 text-gray-500" />
                  <div className="bg-yellow-600 px-3 py-2 rounded">
                    CLI Subprocess<br/>
                    <span className="text-xs opacity-75">Fallback</span>
                  </div>
                  <ArrowRight className="w-4 h-4 text-gray-500" />
                  <div className="bg-red-600 px-3 py-2 rounded">
                    Python Native<br/>
                    <span className="text-xs opacity-75">Always available</span>
                  </div>
                </div>
              </div>

              {/* Key Files */}
              <div className="bg-gray-700 rounded-lg p-4">
                <h3 className="font-medium mb-3">Key Files</h3>
                <div className="space-y-2 font-mono text-sm">
                  <div className="flex justify-between">
                    <span className="text-blue-400">rsk_bridge.py</span>
                    <span className="text-gray-400">~2500 lines, 35 exports</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-blue-400">forge_bridge.py</span>
                    <span className="text-gray-400">RustForge MCP bridge</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-blue-400">__init__.pyi</span>
                    <span className="text-gray-400">Type stubs for IDE</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Section 4: Skill Compliance Levels */}
        {(activeLayer === 'all' || activeLayer === 'skills') && (
          <div className="col-span-6 bg-gray-800 rounded-xl p-6">
            <h2 className="text-xl font-semibold mb-4 flex items-center gap-2">
              <Shield className="w-5 h-5 text-purple-500" />
              Skill Compliance Levels (229 Total)
            </h2>
            <div className="space-y-3">
              {complianceLevels.map(level => (
                <div key={level.level} className="bg-gray-700 rounded-lg p-3">
                  <div className="flex justify-between items-center">
                    <div className="flex items-center gap-2">
                      <span className="text-2xl">{level.badge}</span>
                      <span className="font-medium">{level.level}</span>
                    </div>
                    <div className="flex items-center gap-4">
                      <div className="w-32 bg-gray-600 rounded-full h-2">
                        <div
                          className="bg-orange-500 rounded-full h-2"
                          style={{ width: `${(level.count / 229) * 100}%` }}
                        />
                      </div>
                      <span className="text-sm text-gray-400 w-12 text-right">
                        {level.count}
                      </span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Section 5: Data Flow Timeline */}
        {activeLayer === 'all' && (
          <div className="col-span-12 bg-gray-800 rounded-xl p-6">
            <h2 className="text-xl font-semibold mb-4 flex items-center gap-2">
              <Activity className="w-5 h-5 text-green-500" />
              Data Flow Timeline
            </h2>
            <div className="flex justify-between items-center">
              {dataFlow.map((step, idx) => (
                <React.Fragment key={step.stage}>
                  <div className="flex flex-col items-center text-center max-w-32">
                    <div className="w-12 h-12 rounded-full bg-gray-700 flex items-center justify-center mb-2">
                      <step.icon className="w-6 h-6 text-orange-500" />
                    </div>
                    <span className="font-medium text-sm">{step.stage}</span>
                    <span className="text-xs text-gray-400 mt-1">{step.desc}</span>
                  </div>
                  {idx < dataFlow.length - 1 && (
                    <ArrowRight className="w-6 h-6 text-gray-600 flex-shrink-0" />
                  )}
                </React.Fragment>
              ))}
            </div>
          </div>
        )}

        {/* Section 6: Architecture Diagram (ASCII) */}
        {activeLayer === 'all' && (
          <div className="col-span-12 bg-gray-800 rounded-xl p-6">
            <h2 className="text-xl font-semibold mb-4 flex items-center gap-2">
              <Server className="w-5 h-5 text-cyan-500" />
              System Architecture
            </h2>
            <pre className="text-xs font-mono text-green-400 bg-gray-900 p-4 rounded-lg overflow-x-auto">
{`┌─────────────────────────────────────────────────────────────────────────────┐
│                              CLAUDE CODE CLI                                 │
│                    (User Interface / Conversation Layer)                     │
└─────────────────────────────────────────┬───────────────────────────────────┘
                                          │
                                          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           SKILL FRAMEWORK (KSB)                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   229 Skills │  │   SMST v2    │  │  Compliance  │  │    Hooks     │     │
│  │  (SKILL.md)  │  │  Validation  │  │    Levels    │  │    System    │     │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────────────────────┬───────────────────────────────────┘
                                          │
                                          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         PYTHON BRIDGE LAYER                                  │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │  rsk_bridge.py                                                         │ │
│  │  ├── PyO3 Priority Path (11-70x speedup)                              │ │
│  │  ├── CLI Fallback Path (subprocess)                                    │ │
│  │  └── Python Native Fallback (always available)                        │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│  ┌────────────────────────┐  ┌────────────────────────┐                     │
│  │    forge_bridge.py     │  │   51 Shared Utilities  │                     │
│  └────────────────────────┘  └────────────────────────┘                     │
└─────────────────────────────────────────┬───────────────────────────────────┘
                                          │
                          ┌───────────────┴───────────────┐
                          │         PyO3 FFI              │
                          └───────────────┬───────────────┘
                                          │
                                          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                        RSK RUST KERNEL (v0.5.0)                              │
│                                                                              │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐           │
│  │  levenshtein │ │   graph     │ │    yaml     │ │  taxonomy   │           │
│  │   (69x)     │ │   (DAG)     │ │   (29x)     │ │   (O(1))    │           │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘           │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐           │
│  │text_processor│ │code_generator│ │execution_eng│ │state_manager│           │
│  │   (15x)     │ │  (tests)    │ │   (DAG)     │ │ (checkpoint)│           │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘           │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐           │
│  │   crypto    │ │ compression │ │  telemetry  │ │    math     │           │
│  │   (20x)     │ │   (gzip)    │ │  (tracing)  │ │ (variance)  │           │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘           │
│                                                                              │
│  34 PyO3 Functions │ 265 Tests │ 14 Modules │ ~300KB Source                 │
└─────────────────────────────────────────────────────────────────────────────┘`}
            </pre>
          </div>
        )}

        {/* Section 7: Tech Stack Summary */}
        <div className="col-span-12 bg-gray-800 rounded-xl p-6">
          <h2 className="text-xl font-semibold mb-4 flex items-center gap-2">
            <Package className="w-5 h-5 text-orange-500" />
            Tech Stack Summary
          </h2>
          <div className="grid grid-cols-4 gap-4">
            <div className="bg-gray-700 rounded-lg p-4">
              <h3 className="font-medium text-orange-400 mb-2">Rust Layer</h3>
              <ul className="text-sm text-gray-300 space-y-1">
                <li>• Rust 2024 Edition (1.85+)</li>
                <li>• PyO3 0.23 bindings</li>
                <li>• serde serialization</li>
                <li>• PHF perfect hash tables</li>
                <li>• Polars DataFrames</li>
              </ul>
            </div>
            <div className="bg-gray-700 rounded-lg p-4">
              <h3 className="font-medium text-blue-400 mb-2">Python Layer</h3>
              <ul className="text-sm text-gray-300 space-y-1">
                <li>• Python 3.12+</li>
                <li>• TypedDict types</li>
                <li>• pathlib paths</li>
                <li>• PyYAML fallback</li>
                <li>• subprocess CLI</li>
              </ul>
            </div>
            <div className="bg-gray-700 rounded-lg p-4">
              <h3 className="font-medium text-green-400 mb-2">Build Tools</h3>
              <ul className="text-sm text-gray-300 space-y-1">
                <li>• maturin wheel builder</li>
                <li>• cargo workspaces</li>
                <li>• criterion benchmarks</li>
                <li>• pre-commit hooks</li>
                <li>• Git versioning</li>
              </ul>
            </div>
            <div className="bg-gray-700 rounded-lg p-4">
              <h3 className="font-medium text-purple-400 mb-2">Infrastructure</h3>
              <ul className="text-sm text-gray-300 space-y-1">
                <li>• ~/.claude/.venv</li>
                <li>• ~/.claude/skills/</li>
                <li>• ~/.claude/.checkpoints/</li>
                <li>• manylinux wheels</li>
                <li>• JSON persistence</li>
              </ul>
            </div>
          </div>
        </div>

        {/* Section 8: Module Detail Panel */}
        {activeModule && (
          <div className="col-span-12 bg-orange-900/30 border border-orange-500 rounded-xl p-6">
            <h2 className="text-xl font-semibold mb-4">
              Module Detail: <span className="font-mono text-orange-400">{activeModule}</span>
            </h2>
            {(() => {
              const mod = rskModules.find(m => m.name === activeModule);
              return mod ? (
                <div className="grid grid-cols-4 gap-4">
                  <div className="bg-gray-800 rounded-lg p-4">
                    <div className="text-sm text-gray-400">Size</div>
                    <div className="text-2xl font-bold">{mod.size}</div>
                  </div>
                  <div className="bg-gray-800 rounded-lg p-4">
                    <div className="text-sm text-gray-400">Functions</div>
                    <div className="text-2xl font-bold">{mod.functions}</div>
                  </div>
                  <div className="bg-gray-800 rounded-lg p-4">
                    <div className="text-sm text-gray-400">Criticality</div>
                    <div className="mt-1"><CriticalityBadge level={mod.criticality} /></div>
                  </div>
                  <div className="bg-gray-800 rounded-lg p-4">
                    <div className="text-sm text-gray-400">Source Path</div>
                    <div className="font-mono text-xs text-gray-300 truncate">
                      src/modules/{activeModule}.rs
                    </div>
                  </div>
                </div>
              ) : null;
            })()}
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="mt-8 text-center text-gray-500 text-sm">
        Generated by Architecture Generator | RSK v0.5.0 | {new Date().toISOString().split('T')[0]}
      </div>
    </div>
  );
};

export default RSKArchitectureWireframe;
