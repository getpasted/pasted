import React, { useState, useEffect } from 'react';
import { Operation } from '../types';
import { Wrench, Sparkles, Trash2, Edit3, Code2, Play } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { OperationEditorModal, CATEGORIES } from './OperationEditorModal';
import { startWindowDrag } from '../utils/windowDrag';
import { TransformLibraryToolbar } from './TransformLibraryToolbar';

interface OperationsManagerProps {
  isEmbedded?: boolean;
  onOpenCreateModal?: () => void;
}

export const OperationsManager: React.FC<OperationsManagerProps> = ({
  isEmbedded = false,
  onOpenCreateModal,
}) => {
  const [operations, setOperations] = useState<Operation[]>([]);
  const [activeCategory, setActiveCategory] = useState<string>('All');
  const [selectedOperationForEdit, setSelectedOperationForEdit] = useState<Operation | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [testText, setTestText] = useState('Hello Pasted Operation Library! :) https://example.com?utm_source=test');
  const [testResult, setTestResult] = useState('');

  const fetchOperations = async () => {
    try {
      const res = await invoke<Operation[]>('get_operations');
      setOperations(res);
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    fetchOperations();
  }, []);

  const handleOpenCreate = () => {
    setSelectedOperationForEdit(null);
    setIsModalOpen(true);
  };

  const handleOpenEdit = (op: Operation) => {
    setSelectedOperationForEdit(op);
    setIsModalOpen(true);
  };

  const handleDelete = async (id: number) => {
    try {
      await invoke('delete_operation', { id });
      fetchOperations();
    } catch (e) {
      console.error(e);
    }
  };

  const handleTestOperation = async (opType: string, config: string | null) => {
    try {
      const res = await invoke<string>('transform_text', {
        input: testText,
        filterType: opType,
        config,
      });
      setTestResult(res);
    } catch (e) {
      console.error(e);
    }
  };

  const dynamicCategories = Array.from(
    new Set([...CATEGORIES, ...operations.map((o) => o.category).filter(Boolean)])
  );

  const filteredOps = activeCategory === 'All'
    ? operations
    : operations.filter((o) => o.category === activeCategory);

  const content = (
    <div className="space-y-6">
      {!isEmbedded && (
        <div onMouseDown={startWindowDrag} className="theme-divider flex items-center justify-between pb-4 border-b">
          <div>
            <h2 className="text-lg font-bold theme-title flex items-center space-x-2">
              <Wrench className="transform-accent operations w-5 h-5 opacity-70" />
              <span>Operations Library</span>
            </h2>
            <p className="text-xs theme-text-muted mt-1">
              Build and manage reusable regex replacements, built-in engine keys, and shell script operations.
            </p>
          </div>

          <button
            onClick={onOpenCreateModal || handleOpenCreate}
            className="theme-primary-button flex items-center space-x-2 px-4 py-2 border text-xs font-bold rounded-xl shadow-lg active:scale-95 transition-[background-color,transform]"
          >
            <Sparkles className="w-4 h-4" />
            <span>+ New Operation</span>
          </button>
        </div>
      )}

      {/* Sticky Sandbox */}
      <div className="sticky-filter-sandbox filter-sandbox-card sticky top-0 p-4 rounded-xl border space-y-3 shadow-xl backdrop-blur-xl">
        <div className="flex items-center justify-between">
          <h3 className="filter-sandbox-heading operations text-xs font-semibold uppercase tracking-wider flex items-center space-x-1.5">
            <Play className="w-3.5 h-3.5" />
            <span>Operation Sandbox</span>
          </h3>
          <span className="text-[10px] theme-text-muted">Click any operation card to test live</span>
        </div>
        <div className="grid grid-cols-2 gap-4 text-xs font-mono">
          <div>
            <label className="block theme-text-muted mb-1 font-sans">Input Text:</label>
            <textarea
              value={testText}
              onChange={(e) => setTestText(e.target.value)}
              className="w-full h-24 theme-input border rounded-lg p-2.5 focus:outline-none text-xs"
            />
          </div>
          <div>
            <label className="filter-sandbox-output-label block mb-1 font-sans font-semibold">Output Preview:</label>
            <div className="filter-sandbox-output w-full h-24 theme-input border rounded-lg p-2.5 overflow-y-auto whitespace-pre-wrap">
              {testResult || 'Click any operation card below to test...'}
            </div>
          </div>
        </div>
      </div>

      {/* Category Pills Filter */}
      <TransformLibraryToolbar
        accent="operations"
        createLabel="New Operation"
        onCreate={onOpenCreateModal || handleOpenCreate}
      >
        <button
          onClick={() => setActiveCategory('All')}
          className={`transform-category-pill operations ui-pill px-3 py-1.5 text-xs font-semibold whitespace-nowrap transition-colors ${activeCategory === 'All' ? 'is-active shadow' : ''}`}
        >
          All Operations ({operations.length})
        </button>
        {dynamicCategories.map((cat) => {
          const count = operations.filter((o) => o.category === cat).length;
          return (
            <button
              key={cat}
              onClick={() => setActiveCategory(cat)}
              className={`transform-category-pill operations ui-pill px-3 py-1.5 text-xs font-semibold whitespace-nowrap transition-colors ${activeCategory === cat ? 'is-active shadow' : ''}`}
            >
              {cat} ({count})
            </button>
          );
        })}
      </TransformLibraryToolbar>

      {/* Operations Grid */}
      <div className="space-y-3">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          {filteredOps.map((op) => (
            <div
              key={op.id}
              onClick={() => handleTestOperation(op.op_type, op.config)}
              className="transform-card operations group p-3.5 theme-card-idle rounded-xl border cursor-pointer transition-[background-color,border-color,box-shadow,transform] flex items-center justify-between shadow-md"
            >
              <div className="flex items-center space-x-3 truncate pr-2">
                <div className="p-2 rounded-lg theme-badge border shrink-0">
                  <Code2 className="transform-accent operations w-4 h-4" />
                </div>
                <div className="truncate">
                  <h4 className="text-xs font-bold theme-text-main truncate">{op.name}</h4>
                  <div className="flex items-center space-x-2 mt-0.5">
                    <span className="transform-tag operations text-[10px] font-mono px-1.5 py-0.5 rounded border">
                      {op.op_type}
                    </span>
                    <span className="theme-badge text-[10px] px-1.5 py-0.5 rounded border">
                      {op.category}
                    </span>
                  </div>
                </div>
              </div>

              <div className="flex items-center space-x-1 shrink-0">
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleOpenEdit(op);
                  }}
                  className="theme-icon-button p-1.5 border rounded-md transition-colors"
                  title="Edit Operation"
                >
                  <Edit3 className="w-4 h-4" />
                </button>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDelete(op.id);
                  }}
                  className="theme-icon-button theme-danger-text p-1.5 border rounded-md transition-colors"
                  title="Delete Operation"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Editor Modal */}
      <OperationEditorModal
        operation={selectedOperationForEdit}
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onSaveSuccess={fetchOperations}
      />
    </div>
  );

  return isEmbedded ? (
    content
  ) : (
    <div className="tools-page operations-page flex-1 h-screen overflow-y-auto p-6 space-y-6 select-none filter-manager-wrapper">
      {content}
    </div>
  );
};
