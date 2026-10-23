import {
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormDescription,
} from '@/components/ui/form';
import { Switch } from '@/components/ui/switch';
import { Control } from 'react-hook-form'; // Import Control type

type IInpuItemProps = {
  name: string;
  placeholder?: string;
  control: Control<any>; // Add control prop of type Control
  label?: string;
  value?: string | number;
  formStyle?: string;
  formLabelStyle?: string;
  formControlStyle?: string;
  formMessageStyle?: string;
  defaultValue?: string | number;
  disabled?: boolean;
  description?: string;
  isFormDescription?: boolean;
};

export const SwitchInput = ({
  control,
  label,
  description,
  name,
  isFormDescription = true,
  formStyle,
}: IInpuItemProps) => {
  return (
    <FormField
      control={control}
      name={name}
      render={({ field }) => (
        <FormItem
          className={`flex flex-row items-center justify-between gap-6 space-y-0 py-2.5 ${formStyle ?? ''}`}
        >
          <div className="space-y-0.5">
            <FormLabel>{label}</FormLabel>
            {isFormDescription && (
              <FormDescription>{description}</FormDescription>
            )}
          </div>
          <FormControl>
            <Switch
              checked={field.value}
              onCheckedChange={field.onChange}
            />
          </FormControl>
        </FormItem>
      )}
    />
  );
};
