import * as SelectPrimitive from '@radix-ui/react-select';
import { Check, ChevronDown } from 'lucide-react';
import React from 'react';

export const CustomSelect = React.forwardRef<
  HTMLButtonElement,
  { value: string | number; onChange: (v: string) => void; options: { value: string | number, label: string }[]; placeholder?: string }
>(({ value, onChange, options, placeholder }, ref) => {
  return (
    <SelectPrimitive.Root value={String(value)} onValueChange={onChange}>
      <SelectPrimitive.Trigger ref={ref} className="custom-select-trigger">
        <SelectPrimitive.Value placeholder={placeholder} />
        <SelectPrimitive.Icon>
          <ChevronDown size={16} />
        </SelectPrimitive.Icon>
      </SelectPrimitive.Trigger>

      <SelectPrimitive.Portal>
        <SelectPrimitive.Content className="custom-select-content">
          <SelectPrimitive.Viewport style={{ padding: 5 }}>
            {options.map((opt) => (
              <SelectPrimitive.Item key={opt.value} value={String(opt.value)} className="custom-select-item">
                <SelectPrimitive.ItemText>{opt.label}</SelectPrimitive.ItemText>
                <SelectPrimitive.ItemIndicator style={{ position: 'absolute', left: 5 }}>
                  <Check size={14} />
                </SelectPrimitive.ItemIndicator>
              </SelectPrimitive.Item>
            ))}
          </SelectPrimitive.Viewport>
        </SelectPrimitive.Content>
      </SelectPrimitive.Portal>
    </SelectPrimitive.Root>
  );
});
