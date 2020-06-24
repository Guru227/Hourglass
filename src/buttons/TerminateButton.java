package buttons;

import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;

//This button terminates the program
public class TerminateButton extends MenuButton {
	/**
	 * 
	 */
	private static final long serialVersionUID = -7612092625511056530L;

	public TerminateButton(String m){
		super();
		setText(m);		//Set message on button
		addActionListener(new TerminateOnClick());		//Add action listeners
	}
}
//Terminates program (Timer stops running)
class TerminateOnClick implements ActionListener{
  public void actionPerformed(ActionEvent e) {
  	System.exit(0);
  }		
}
